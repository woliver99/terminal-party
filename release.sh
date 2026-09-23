#!/usr/bin/env bash
set -e

# ==============================================================================
# Terminal Party - Local Build & Release Script 🚀
# Builds static, portable Linux x86_64 binaries on NixOS and uploads via gh CLI
# ==============================================================================

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
    read -rp "Enter release version tag (e.g. v1.0.0): " VERSION
fi

if [ -z "$VERSION" ]; then
    echo "❌ Error: Version tag is required."
    exit 1
fi

echo "=================================================="
echo " 🎉 Terminal Party Release Builder ($VERSION)"
echo "=================================================="

# 1. Verify GitHub CLI authentication
if ! command -v gh >/dev/null 2>&1; then
    echo "❌ Error: 'gh' (GitHub CLI) is not installed."
    exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "❌ Error: 'gh' is not authenticated. Please run 'gh auth login' first."
    exit 1
fi

# 2. Detect container engine (podman preferred on NixOS, fallback to docker)
if command -v podman >/dev/null 2>&1; then
    ENGINE="podman"
elif command -v docker >/dev/null 2>&1; then
    ENGINE="docker"
else
    echo "❌ Error: Neither 'podman' nor 'docker' was found."
    echo "A container engine is needed on NixOS to produce static musl binaries for other Linux systems."
    exit 1
fi

echo "🐳 Using container engine: $ENGINE"

# 3. Build static release binaries inside rust:alpine
echo "⚙️  Compiling termcraft and party-chat for target x86_64 (static musl)..."
$ENGINE run --rm -v "$PWD":/workspace -w /workspace rust:alpine \
    cargo build --release --workspace

# 4. Verify outputs exist
if [ ! -f "target/release/termcraft" ] || [ ! -f "target/release/party-chat" ]; then
    echo "❌ Error: Build finished but expected binaries were not found in target/release/."
    exit 1
fi

# 5. Assemble distribution package
echo "📦 Assembling release bundle..."
rm -rf dist terminal-party-linux-x86_64.tar.gz
mkdir -p dist

cp target/release/termcraft dist/
cp target/release/party-chat dist/
cp party.sh dist/
echo "$VERSION" > dist/version.txt

chmod 755 dist/termcraft dist/party-chat dist/party.sh

# Strip binaries inside container to keep size minimal
$ENGINE run --rm -v "$PWD":/workspace -w /workspace rust:alpine \
    strip dist/termcraft dist/party-chat || true

tar -czf terminal-party-linux-x86_64.tar.gz -C dist .
rm -rf dist

echo "✅ Package created: terminal-party-linux-x86_64.tar.gz ($(du -h terminal-party-linux-x86_64.tar.gz | cut -f1))"

# 6. Upload release using GitHub CLI
echo "🚀 Uploading release $VERSION to GitHub..."
RELEASE_NOTES="## 🎉 Terminal Party $VERSION

3D Multiplayer Minecraft Engine powered by **TermCraft ([@vikvang](https://github.com/vikvang))**

### Quick Start on any shared SSH machine:
\`\`\`bash
bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/woliver99/terminal-party/main/party.sh)\"
\`\`\`

Or simply copy \`party.sh\` to the server and run:
\`\`\`bash
chmod +x party.sh
./party.sh
\`\`\`"

if gh release view "$VERSION" >/dev/null 2>&1; then
    echo "Release $VERSION exists, updating assets..."
    gh release upload "$VERSION" terminal-party-linux-x86_64.tar.gz party.sh --clobber
else
    gh release create "$VERSION" \
        terminal-party-linux-x86_64.tar.gz \
        party.sh \
        --title "Terminal Party $VERSION" \
        --notes "$RELEASE_NOTES"
fi

echo "=================================================="
echo " ✨ Release $VERSION successfully published!"
echo " 🔗 https://github.com/woliver99/terminal-party/releases/tag/$VERSION"
echo "=================================================="
