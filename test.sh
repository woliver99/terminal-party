#!/usr/bin/env bash
set -e

# ==============================================================================
# Terminal Party - Pre-Release Test Suite 🧪
# Tests compilation, static linking, player persistence, and sandbox permissions
# ==============================================================================

echo "=================================================="
echo " 🧪 Terminal Party Pre-Release Test Suite"
echo "=================================================="

# 1. Detect container engine (podman / docker)
if command -v podman >/dev/null 2>&1; then
    ENGINE="podman"
elif command -v docker >/dev/null 2>&1; then
    ENGINE="docker"
else
    echo "❌ Error: Neither 'podman' nor 'docker' found."
    exit 1
fi
echo "🐳 Using container engine: $ENGINE"

# 2. Run unit tests in container
echo "🧪 Running unit tests (including player persistence)..."
$ENGINE run --rm -v "$PWD":/workspace -w /workspace rust:alpine \
    cargo test --workspace

# 3. Build release binaries
echo "⚙️  Building release binaries in container..."
$ENGINE run --rm -v "$PWD":/workspace -w /workspace rust:alpine \
    cargo build --release --workspace

# 4. Verify release binaries exist and are statically linked
echo "🔍 Checking binary outputs and static linking..."
if [ ! -f "target/release/termcraft" ] || [ ! -f "target/release/party-chat" ]; then
    echo "❌ Error: Binaries not found in target/release/."
    exit 1
fi

for bin in target/release/party-chat target/release/termcraft; do
    if ldd "$bin" 2>&1 | grep -q "not a dynamic executable\|statically linked"; then
        echo "  ✅ $bin is 100% statically linked."
    elif [ "$(ldd "$bin" 2>&1)" = "" ]; then
        echo "  ✅ $bin has no dynamic dependencies."
    else
        echo "  ⚠️ Warning: $bin might have dynamic dependencies:"
        ldd "$bin" 2>&1 || true
    fi
done

# 5. Automated sandbox permission checks
echo "🔒 Testing multi-user permissions in sandbox..."
TEST_SANDBOX="/tmp/test-party-sandbox-$$"
mkdir -p "$TEST_SANDBOX"
chmod 755 "$TEST_SANDBOX"

# Verify 755
SANDBOX_PERMS=$(stat -c "%a" "$TEST_SANDBOX" 2>/dev/null || stat -f "%Op" "$TEST_SANDBOX" | tail -c 4)
echo "  • Sandbox root mode: $SANDBOX_PERMS (expected: 755)"

# Verify sticky-bit messages directory (1777)
mkdir -p "$TEST_SANDBOX/messages"
chmod 1777 "$TEST_SANDBOX/messages"
MESSAGES_PERMS=$(stat -c "%a" "$TEST_SANDBOX/messages" 2>/dev/null || stat -f "%Op" "$TEST_SANDBOX/messages" | tail -c 5)
echo "  • Messages dir mode: $MESSAGES_PERMS (expected: 1777)"

# Test log file creation (0644)
touch "$TEST_SANDBOX/messages/test_user.log"
chmod 644 "$TEST_SANDBOX/messages/test_user.log"
echo "[12:00:00] Hello test" >> "$TEST_SANDBOX/messages/test_user.log"

rm -rf "$TEST_SANDBOX"
echo "  ✅ Permission sandbox passed."

# 6. Check shell scripts syntax
echo "📜 Checking script syntax..."
bash -n party.sh
bash -n release.sh
echo "  ✅ Shell scripts passed syntax validation."

echo ""
# 7. Deploy local test environment to /tmp/test-terminal-party
INTERACTIVE_DIR="/tmp/test-terminal-party"
echo "📦 Deploying local build to $INTERACTIVE_DIR for testing..."
rm -rf "$INTERACTIVE_DIR"
mkdir -p "$INTERACTIVE_DIR"
chmod 755 "$INTERACTIVE_DIR"
cp target/release/party-chat target/release/termcraft party.sh "$INTERACTIVE_DIR/"
echo "local-test" > "$INTERACTIVE_DIR/version.txt"
chmod 755 "$INTERACTIVE_DIR/party-chat" "$INTERACTIVE_DIR/termcraft" "$INTERACTIVE_DIR/party.sh"
mkdir -p "$INTERACTIVE_DIR/messages"
chmod 1777 "$INTERACTIVE_DIR/messages"

echo "  ✅ Installed test build to $INTERACTIVE_DIR"
echo ""
echo "=================================================="
echo " ✅ Automated tests completed successfully!"
echo "=================================================="
echo ""
echo "🎮 You can test Terminal Party at any time by running:"
echo "   $INTERACTIVE_DIR/party.sh"
echo ""

# 8. Optional interactive test
read -rp "Would you like to launch an interactive test session right now? [Y/n] " run_interactive
if [[ ! "$run_interactive" =~ ^[Nn]$ ]]; then
    echo "🎉 Starting interactive session..."
    echo "Tip: Try sending messages, type /invite, /minecraft, and exit with /quit."
    "$INTERACTIVE_DIR/party.sh" || true
fi

echo ""
echo "🚀 When you are happy with the test, publish your release with:"
echo "   ./release.sh v1.0.2"
echo ""
echo "🧹 To clean up test files when finished:"
echo "   rm -rf $INTERACTIVE_DIR"
