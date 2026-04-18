#!/bin/bash

SERVICE_DIR="$HOME/.config/systemd/user"

systemctl --user stop pompiliusd.service 2>/dev/null
systemctl --user stop rclone-daemon.service 2>/dev/null
systemctl --user disable pompiliusd.service 2>/dev/null
systemctl --user disable rclone-daemon.service 2>/dev/null

rm -f "$SERVICE_DIR/pompiliusd.service"
rm -f "$SERVICE_DIR/rclone-daemon.service"

systemctl --user daemon-reload
systemctl --user reset-failed

cargo clean
