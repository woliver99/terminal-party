#!/usr/bin/env bash
set -e

# ==============================================================================
# Terminal Party Launcher 🎉
# Created by Oliver (woliver99)
# https://github.com/woliver99/terminal-party
# ==============================================================================

PARTY_DIR="${PARTY_DIR:-/tmp/terminal-party}"
MESSAGES_DIR="$PARTY_DIR/messages"
REPO="woliver99/terminal-party"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/terminal-party-linux-x86_64.tar.gz"

# Safe umask: read/exec for all, write only for creator
umask 022

if [ ! -d "$PARTY_DIR" ]; then
    echo "🎉 Setting up Terminal Party in $PARTY_DIR..."
    mkdir -p "$PARTY_DIR"
    chmod 755 "$PARTY_DIR" 2>/dev/null || true

    TMP_TAR="/tmp/terminal-party-$$.tar.gz"
    echo "📦 Downloading latest release from $REPO..."
    
    DOWNLOAD_SUCCESS=0
    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL -o "$TMP_TAR" "$RELEASE_URL"; then
            DOWNLOAD_SUCCESS=1
        fi
    fi

    if [ "$DOWNLOAD_SUCCESS" -ne 1 ] && command -v wget >/dev/null 2>&1; then
        if wget -q -O "$TMP_TAR" "$RELEASE_URL"; then
            DOWNLOAD_SUCCESS=1
        fi
    fi

    if [ "$DOWNLOAD_SUCCESS" -eq 1 ] && [ -s "$TMP_TAR" ]; then
        echo "📂 Extracting game and chat binaries..."
        tar -xzf "$TMP_TAR" -C "$PARTY_DIR"
        rm -f "$TMP_TAR"
        chmod 755 "$PARTY_DIR/party-chat" "$PARTY_DIR/termcraft" 2>/dev/null || true
    else
        echo "⚠️  Could not download prebuilt release from GitHub."
        # If running from repository checkout, copy local build if present
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
        if [ -x "$SCRIPT_DIR/target/release/party-chat" ] && [ -x "$SCRIPT_DIR/target/release/termcraft" ]; then
            echo "📦 Copying locally built binaries from $SCRIPT_DIR..."
            cp "$SCRIPT_DIR/target/release/party-chat" "$PARTY_DIR/"
            cp "$SCRIPT_DIR/target/release/termcraft" "$PARTY_DIR/"
            echo "local-dev" > "$PARTY_DIR/version.txt"
            chmod 755 "$PARTY_DIR/party-chat" "$PARTY_DIR/termcraft" 2>/dev/null || true
        else
            echo "❌ Error: Release download failed and local release binaries not found."
            echo "Please check your network connection or build locally with 'cargo build --release'."
            exit 1
        fi
    fi
fi

# Ensure shared messages directory exists with sticky-bit permissions (1777)
# Like /tmp itself: any user can create their file, but only they can modify or delete it!
mkdir -p "$MESSAGES_DIR" 2>/dev/null || true
chmod 1777 "$MESSAGES_DIR" 2>/dev/null || true

# Launch the chat app
if [ -x "$PARTY_DIR/party-chat" ]; then
    export PARTY_DIR
    exec "$PARTY_DIR/party-chat" "$@"
else
    echo "❌ Error: $PARTY_DIR/party-chat not found or not executable."
    exit 1
fi
