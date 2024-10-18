# (SOD) sshx-on-demand

Uses [`sshx`](https://sshx.io/) to enable any remote host to have a web-based
terminal interface. **Make sure your Next.JS is app is behind a robust auth
solution - this gives complete access to anyone who wants to take over your
server**.

There are two parts:
- A Rust binary that runs persistently on the server that listens/ reads from a named pipe.
- A Next.JS server that calls the Rust binary.

It's architected this way so that the Next.JS app can run
in a Dockerfile independent of the host.

Note: If you're testing on a mac, the docker approach won't work due to
nuances with named pipes I don't understand.

# Installation
```
## host_app/

# Install binary
VER=$(curl --silent -qI https://github.com/cryingpotat0/sshx-on-demand/releases/latest | awk -F '/' '/^location/ {print  substr($NF, 1, length($NF)-1)}');
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cryingpotat0/sshx-on-demand/releases/download/$VER/sshx-on-demand-installer.sh | sh

# Setup systemd
cp sshx-on-demand.service /etc/systemd/system/sshx-on-demand.service
sudo systemctl daemon-reload
sudo systemctl start sshx-on-demand
sudo systemctl enable sshx-on-demand

# Check
sudo systemctl status sshx-backend

# Logs
sudo journalctl -u sshx-backend

## frontend/
docker-compose up -d
```

# Testing
```
# frontend/
npm run dev 

# host_app/
cargo run
```

# Upgrading
```
# host_app/
sudo systemctl stop sshx-on-demand
VER=$(curl --silent -qI https://github.com/cryingpotat0/sshx-on-demand/releases/latest | awk -F '/' '/^location/ {print  substr($NF, 1, length($NF)-1)}');
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/cryingpotat0/sshx-on-demand/releases/download/$VER/sshx-on-demand-installer.sh | sh

# frontend/
# NOTE: You have to do this after so it picks the right named pipe.
docker-compose down
docker-compose up -d
```
