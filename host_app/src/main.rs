use clap::Parser;
use env_logger::Env;
use log::{error, info};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(default_value = "/tmp/sshx-host-runner-read")]
    reader_pipe_path: String,

    #[arg(default_value = "/tmp/sshx-host-runner-write")]
    writer_pipe_path: String,

    #[arg(default_value_t = 60)]
    idle_timeout_secs: u64,
}

struct AppState {
    child: Option<Child>,
    last_keepalive: Option<std::time::Instant>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // Check if sshx is available
    if Command::new("sshx")
        .arg("--version")
        .output()
        .await
        .is_err()
    {
        error!("sshx command not found. Please make sure it's installed and available in PATH.");
        std::process::exit(1);
    }

    info!(
        "SSHX idle timeout set to {} seconds",
        args.idle_timeout_secs
    );

    let state = Arc::new(Mutex::new(AppState {
        child: None,
        last_keepalive: None,
    }));

    for pipe_path in [args.reader_pipe_path.clone(), args.writer_pipe_path.clone()].iter() {
        if let Err(e) = std::fs::remove_file(pipe_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                error!("Failed to remove existing pipe: {}", e);
                return Err(e.into());
            }
        }
        if let Err(e) = unix_named_pipe::create(pipe_path, Some(0o666)) {
            error!("Failed to create named pipe: {}", e);
            return Err(e.into());
        }
        info!("Named pipe opened at: {}", pipe_path);
    }

    loop {
        let mut file = match OpenOptions::new()
            .read(true)
            // .custom_flags(libc::O_NONBLOCK)
            .open(args.reader_pipe_path.clone())
        {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to open named pipe: {}", e);
                continue;
            }
        };

        let mut str = "".to_string();
        match file.read_to_string(&mut str) {
            Ok(_) => {
                if str == "OpenNewConnection" {
                    handle_new_conn_request(&state, &args).await;
                } else if str == "KeepAlive" {
                    handle_keepalive_request(&state, &args).await;
                } else {
                    error!("Unknown command {:?} flushing buffer", str);
                    // Flush buffer.
                    file.read_to_end(&mut Vec::new()).unwrap_or_default();
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                error!("Sleeping: {}", e);
                sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                error!("Error reading from pipe: {}", e);
            }
        }
    }
}

async fn handle_keepalive_request(state: &Arc<Mutex<AppState>>, args: &Args) {
    info!("Received KeepAlive request");
    let mut unlocked_state = state.lock().await;
    unlocked_state.last_keepalive = Some(std::time::Instant::now());

    let mut file = match OpenOptions::new()
        .write(true)
        .open(args.writer_pipe_path.clone())
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open named writer pipe: {}", e);
            return;
        }
    };

    if let Err(e) = file.write_all(b"OK") {
        error!("Failed to write OK to pipe: {}", e);
    }
}

async fn handle_new_conn_request(state: &Arc<Mutex<AppState>>, args: &Args) {
    info!("Received OpenNewConnection request");

    let mut file = match OpenOptions::new()
        .read(true)
        .write(true)
        .open(args.writer_pipe_path.clone())
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open named writer pipe: {}", e);
            return;
        }
    };

    {
        let mut unlocked_state = state.lock().await;
        if let Some(mut existing_child) = unlocked_state.child.take() {
            info!("Killing existing connection");
            existing_child.kill().await.unwrap();
        }
    }

    match run_sshx().await {
        Ok((new_url, child)) => {
            info!("New URL found: {}", new_url);
            {
                let mut unlocked_state = state.lock().await;
                unlocked_state.child = Some(child);
                unlocked_state.last_keepalive = Some(std::time::Instant::now());
            }

            {
                let state_clone = state.clone();
                let idle_timeout_secs = args.idle_timeout_secs;
                tokio::spawn(async move {
                    loop {
                        sleep(Duration::from_secs(60)).await;
                        {
                            let mut unlocked_state = state_clone.lock().await;

                            if let Some(last_keepalive) = unlocked_state.last_keepalive {
                                if last_keepalive.elapsed().as_secs() > idle_timeout_secs {
                                    info!("Idle timeout reached, killing sshx");
                                    if let Some(mut existing_child) = unlocked_state.child.take() {
                                        existing_child.kill().await.unwrap();
                                        return;
                                    }
                                }
                            }
                        }
                    }
                });
            }

            if let Err(e) = file.write_all(new_url.as_bytes()) {
                error!("Failed to write new URL to pipe: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to run sshx: {}", e);
            if let Err(e) = file.write_all(b"ERROR") {
                error!("Failed to write error message to pipe: {}", e);
            }
        }
    }
}

async fn run_sshx() -> Result<(String, Child), Box<dyn std::error::Error>> {
    info!("Spawning sshx");
    let mut child = Command::new("sshx")
        .arg("-q")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

    let mut reader = BufReader::new(stdout).lines();

    Ok((reader.next_line().await?.unwrap(), child))
}
