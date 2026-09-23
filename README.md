# 🎉 Terminal Party

A high-performance, tamper-proof terminal chat app and multiplayer 3D Minecraft launcher designed specifically for shared SSH servers (CS lab machines, school servers, shared VMs).

Built in Rust with zero external runtime dependencies. Runs anywhere on Linux `x86_64`.

---

## ✨ Features

- **Built for Shared SSH Servers**: No root permissions, no `netcat`, and no Python required.
- **Tamper-Proof & Anti-Spoofing Architecture**:
  - Uses a Unix **sticky-bit directory** (`/tmp/terminal-party/messages/`, mode `1777`).
  - Each user writes exclusively to their own log: `messages/<username>.log` (mode `644`).
  - Nobody can modify, delete, or fake messages from another student.
  - Binaries and launcher scripts are strictly read-only (`chmod 755`) to non-creators.
- **3D Multiplayer Minecraft in Terminal**:
  - Type `/minecraft` or `/mc` right in chat to launch into a shared 3D Minecraft world!
  - Automatically launches with the multiplayer rendezvous on **Seed 7** with your username.
  - First player to run hosts the world; everyone else joins automatically on localhost!
  - Chat announces when players enter and leave Minecraft.
  - Returning from Minecraft seamlessly restores your terminal and chat feed.
- **Universal Entrypoint**: Run `party.sh` from any directory or run it directly via curl. If `/tmp/terminal-party` doesn't exist yet, it sets up the shared folder, downloads the release, and joins. If it already exists, it immediately joins the party!

---

## 🚀 Quick Start

### 1. One-Line Launch (on any shared SSH server)
```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/woliver99/terminal-party/main/party.sh)"
```

### 2. Or Copy & Run
You can copy `party.sh` anywhere on the server and run:
```bash
chmod +x party.sh
./party.sh
```

Once installed, anyone on the server can also simply run:
```bash
/tmp/terminal-party/party.sh
```

---

## 🎮 In-Chat Commands

| Command | Description |
|---|---|
| `/minecraft` or `/mc` | Launch into the shared 3D Minecraft world (Seed 7) |
| `/credits` | Show creator and engine credits |
| `/help` | Show available commands |
| `/clear` | Clear the chat feed view |
| `/quit` or `/exit` | Leave the chat session |

---

## 🔒 Security & Architecture

Shared university servers have many concurrent users. Terminal Party is architected with multi-user isolation in mind:

1. **Read-Only Binaries (`755`)**:
   When the first student installs to `/tmp/terminal-party`, the directory and binaries (`party-chat`, `termcraft`) are read-only for all other users. No other user can tamper with, overwrite, or inject malicious code into the executables.

2. **Per-User Sticky-Bit Logging (`1777`)**:
   Messages live in `/tmp/terminal-party/messages/`. Like `/tmp` itself, the sticky bit prevents users from modifying or deleting files owned by other users.
   - Oliver writes to `messages/woliver99.log`.
   - Alice writes to `messages/alice.log`.
   - Bob cannot forge messages as Alice, because Bob has no write access to `alice.log`.
   - The TUI reads all logs and strictly attributes lines to the file owner's username.

---

## 🧪 Testing Before Release

You can test compilation, static linking, player persistence, and sandbox permissions locally at any time:

```bash
./test.sh
```

This runs:
- All unit tests (including player persistence save/load tests).
- Containerized static compilation for `x86_64`.
- Automated permission tests in a mock sandbox directory.
- Optional interactive session in `/tmp/test-terminal-party` to test chat and TermCraft before releasing.

---

## 📦 How to Create Releases

Releases can be built and uploaded whenever you want using the local release script:

```bash
./release.sh v1.0.0
```

The script will:
1. Compile both `termcraft` and `party-chat` from source inside `rust:alpine` via rootless `podman` (or `docker`).
2. Generate 100% statically-linked binaries (`x86_64-unknown-linux-musl`) with no external library dependencies (safe for any Linux distribution).
3. Strip the binaries and package `terminal-party-linux-x86_64.tar.gz`.
4. Upload the release and `party.sh` directly to GitHub using your authenticated `gh` CLI.

---

## 👥 Credits

- **Creator & Maintainer**: **Oliver** ([@woliver99](https://github.com/woliver99))
- **3D Terminal Minecraft Engine**: **TermCraft** by [@vikvang](https://github.com/vikvang) ([termcraft](https://github.com/vikvang/termcraft))
