#!/usr/bin/env bash
set -e

# ==============================================================================
# Terminal Party Launcher 🎉
# Created by Oliver (woliver99)
# https://github.com/woliver99/terminal-party
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"

# 1. Resolve target directory:
# - If PARTY_DIR is explicitly passed via environment, honor it.
# - Otherwise, if party.sh is running alongside an existing party-chat binary, use SCRIPT_DIR.
# - Otherwise, default to /tmp/terminal-party.
if [ -n "$PARTY_DIR" ]; then
    TARGET_DIR="$PARTY_DIR"
elif [ -x "$SCRIPT_DIR/party-chat" ]; then
    TARGET_DIR="$SCRIPT_DIR"
else
    TARGET_DIR="/tmp/terminal-party"
fi
PARTY_DIR="$TARGET_DIR"
DATA_DIR="$PARTY_DIR/data"
MESSAGES_DIR="$DATA_DIR/messages"
REPO="woliver99/terminal-party"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/terminal-party-linux-x86_64.tar.gz"

# Safe umask: read/exec for all, write only for creator
umask 022

# Check if current user is owner of directory or root
is_dir_owner() {
    local dir="$1"
    [ -O "$dir" ] || [ "$(id -u)" -eq 0 ]
}

show_help() {
    cat <<EOF
Terminal Party Launcher 🎉

Usage:
  party.sh [OPTIONS]

Options:
  --update, -u    Update Terminal Party binaries in $PARTY_DIR (owner only)
  --version, -v   Show installed version
  --help, -h      Show this help message

In-Party Commands:
  /minecraft, /mc [creative]  Launch 3D Minecraft (Survival or Creative)
  /creative                   Launch directly into Creative Mode
  /update                     Show instructions to update
  /invite                     Show server invite command
  /quit                       Exit party
EOF
    exit 0
}

show_version() {
    if [ -f "$PARTY_DIR/version.txt" ]; then
        echo "Terminal Party $(cat "$PARTY_DIR/version.txt")"
    else
        echo "Terminal Party (version unrecorded)"
    fi
    exit 0
}

# Handle CLI options
case "${1:-}" in
    --help|-h)
        show_help
        ;;
    --version|-v)
        show_version
        ;;
    --update|-u)
        DO_UPDATE=1
        ;;
    *)
        DO_UPDATE=0
        ;;
esac

# Check ownership if updating an existing directory
if [ "$DO_UPDATE" -eq 1 ]; then
    if [ -d "$PARTY_DIR" ] && ! is_dir_owner "$PARTY_DIR"; then
        DIR_OWNER=$(stat -c '%U' "$PARTY_DIR" 2>/dev/null || stat -f '%Su' "$PARTY_DIR" 2>/dev/null || echo "another user")
        echo "❌ Permission denied: You do not own $PARTY_DIR."
        echo "   Only the directory owner ($DIR_OWNER) or root can update Terminal Party."
        exit 1
    fi
fi

if [ "$DO_UPDATE" -eq 1 ] || [ ! -d "$PARTY_DIR" ] || [ ! -x "$PARTY_DIR/party-chat" ]; then
    if [ "$DO_UPDATE" -eq 1 ]; then
        CURRENT_VER="unknown"
        [ -f "$PARTY_DIR/version.txt" ] && CURRENT_VER="$(cat "$PARTY_DIR/version.txt" 2>/dev/null)"
        echo "🔄 Updating Terminal Party in $PARTY_DIR (current version: $CURRENT_VER)..."
    else
        echo "🎉 Setting up Terminal Party in $PARTY_DIR..."
    fi

    mkdir -p "$PARTY_DIR"
    chmod 755 "$PARTY_DIR" 2>/dev/null || true

    # If running from source repository checkout with built release binaries, copy directly
    if [ -x "$SCRIPT_DIR/target/release/party-chat" ] && [ -x "$SCRIPT_DIR/target/release/termcraft" ]; then
        echo "📦 Using locally built binaries from $SCRIPT_DIR..."
        cp "$SCRIPT_DIR/target/release/party-chat" "$PARTY_DIR/"
        cp "$SCRIPT_DIR/target/release/termcraft" "$PARTY_DIR/"
        [ -f "$SCRIPT_DIR/party.sh" ] && cp "$SCRIPT_DIR/party.sh" "$PARTY_DIR/" 2>/dev/null || true
        echo "local-build" > "$PARTY_DIR/version.txt"
        chmod 755 "$PARTY_DIR/party-chat" "$PARTY_DIR/termcraft" "$PARTY_DIR/party.sh" 2>/dev/null || true
    else
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
            [ -f "$PARTY_DIR/party.sh" ] && chmod 755 "$PARTY_DIR/party.sh" 2>/dev/null || true
        else
            echo "❌ Error: Could not download prebuilt release from GitHub and local build not found."
            exit 1
        fi
    fi

    # Ensure shared data directory and messages directory exist with sticky-bit permissions (1777)
    # Like /tmp itself: any user can create their file, but only they can modify or delete it!
    mkdir -p "$DATA_DIR" "$MESSAGES_DIR" 2>/dev/null || true
    chmod 1777 "$DATA_DIR" "$MESSAGES_DIR" 2>/dev/null || true

    if [ "$DO_UPDATE" -eq 1 ]; then
        NEW_VER="unknown"
        [ -f "$PARTY_DIR/version.txt" ] && NEW_VER="$(cat "$PARTY_DIR/version.txt" 2>/dev/null)"
        echo "✅ Terminal Party successfully updated to $NEW_VER!"
        echo "🎮 Run '$PARTY_DIR/party.sh' to join the party."
        exit 0
    fi
fi

# Ensure shared data directory and messages directory exist with sticky-bit permissions (1777)
mkdir -p "$DATA_DIR" "$MESSAGES_DIR" 2>/dev/null || true
chmod 1777 "$DATA_DIR" "$MESSAGES_DIR" 2>/dev/null || true

# Launch the chat app
if [ -x "$PARTY_DIR/party-chat" ]; then
    export PARTY_DIR
    exec "$PARTY_DIR/party-chat" "$@"
else
    echo "❌ Error: $PARTY_DIR/party-chat not found or not executable."
    exit 1
fi
