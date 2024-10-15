use clap::Parser;
use env_logger::Env;
use log::{error, info};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Simple program to greet a person
#[derive(Parser, Debug)]
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

#[derive(Clone)]
struct AppState {
    current_url: Arc<Mutex<Option<String>>>,
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

    let state = AppState {
        current_url: Arc::new(Mutex::new(None)),
    };

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

        let mut buffer = [0; 4];
        match file.read_exact(&mut buffer) {
            Ok(_) => {
                if &buffer.to_ascii_uppercase() == b"PING" {
                    handle_ping(&state, &args).await;
                } else {
                    error!("Unknown command {:?} flushing buffer", buffer);
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

async fn handle_ping(state: &AppState, args: &Args) {
    info!("Received PING request");
    let mut url = state.current_url.lock().await;

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

    if let Some(existing_url) = url.as_ref() {
        info!("Returning existing URL: {}", existing_url);
        if let Err(e) = file.write_all(existing_url.as_bytes()) {
            error!("Failed to write existing URL to pipe: {}", e);
        }
        return;
    }

    match run_sshx().await {
        Ok(new_url) => {
            *url = Some(new_url.clone());
            info!("New URL found: {}", new_url);
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
    // TODO: should have a way to end a session/ see if it's still active.
}

async fn run_sshx() -> Result<String, Box<dyn std::error::Error>> {
    info!("Spawning sshx");
    let mut child = Command::new("sshx")
        .arg("-q")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

    let mut reader = BufReader::new(stdout).lines();

    Ok(reader.next_line().await?.unwrap())
}
