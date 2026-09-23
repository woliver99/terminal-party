#!/bin/bash

TARGET_DIR="/tmp/shared_chat"

# Create target directory and open permissions
mkdir -p "$TARGET_DIR"
chmod 777 "$TARGET_DIR"

# Copy the existing chat.sh script over
cp chat.sh "$TARGET_DIR/chat.sh"
chmod 777 "$TARGET_DIR/chat.sh"

# Initialize log file
touch "$TARGET_DIR/chat.log"
chmod 666 "$TARGET_DIR/chat.log"

echo "Done! Anyone can run: $TARGET_DIR/chat.sh"
