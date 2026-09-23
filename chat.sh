#!/bin/bash

SHARED_DIR="/tmp/shared_chat"
CHAT_LOG="$SHARED_DIR/chat.log"

# Clean up background tail process when exiting
trap 'kill $(jobs -p) 2>/dev/null; exit' SIGINT SIGTERM EXIT

clear
echo "=========================================="
echo "   Terminal Chat (Press Ctrl+C to quit)   "
echo "=========================================="

# Display previous 15 messages and follow live
tail -n 15 -f "$CHAT_LOG" &

# Read input line-by-line
while true; do
    read -r msg
    if [ -n "$msg" ]; then
        tput cuu1 2>/dev/null
        tput el 2>/dev/null
        echo "[$(date +'%H:%M:%S')] <$(whoami)> $msg" >> "$CHAT_LOG" 
    fi
done
