A simple runner over a named pipe. Usage can be obtained by running `cargo run --release --help`.

Can be tested by opening three terminals, and in order:
- `cargo run`
- `cat < /tmp/sshx-host-runner-write`
- `echo ping > /tmp/sshx-host-runner-read`

