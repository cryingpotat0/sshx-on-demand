# (SOD) sshx-on-demand

Uses [`sshx`](https://sshx.io/) to enable any remote host to have a web-based
terminal interface. **Make sure your Next.JS is app is behind a robust auth
solution - this gives complete access to anyone who wants to take over your
server**.

There are two parts:
- A Rust binary that runs persistently on the server that listens/ reads from a named pipe.
- A Next.JS server that calls the Rust binary.

It's architected this way so that the Next.JS app can run
in a Dockerfile independent of the host process.

