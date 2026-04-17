#!/bin/bash

SERVICE_DIR="$HOME/.config/systemd/user"

systemctl --user stop pompiliusd.service 2>/dev/null
systemctl --user stop rclone-rcd.service 2>/dev/null
systemctl --user disable pompiliusd.service 2>/dev/null
systemctl --user disable rclone-rcd.service 2>/dev/null

rm -f "$SERVICE_DIR/pompiliusd.service"
rm -f "$SERVICE_DIR/rclone-rcd.service"

systemctl --user daemon-reload
systemctl --user reset-failed

cargo clean
