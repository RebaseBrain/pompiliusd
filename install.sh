#!/bin/bash

commands=("cargo" "rclone" "busctl")
for cmd in "${commands[@]}"; do
  if ! command -v $cmd &> /dev/null; then
    echo -e "Error: $cmd is not installed."
    exit 1
  fi
done

cargo build --release

SERVICE_DIR="$HOME/.config/systemd/user"
BINARY_PATH="$(pwd)/target/release/pompiliusd"
RCLONE_BIN="$(which rclone)"
mkdir -p "$SERVICE_DIR"

cat <<EOF > "$SERVICE_DIR/rclone-daemon.service"
[Unit]
Description=Rclone Remote Control Daemon
After=network.target

[Service]
ExecStart=$RCLONE_BIN rcd --rc-addr localhost:5572 --rc-no-auth
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
EOF

cat <<EOF > "$SERVICE_DIR/pompiliusd.service"
[Unit]
Description=Pompilius Daemon
After=rclone-daemon.service

[Service]
ExecStart=$BINARY_PATH
Restart=always
RestartSec=3
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now rclone-daemon.service
systemctl --user enable --now pompiliusd.service

sleep 2
systemctl --user is-active --quiet pompiliusd.service
