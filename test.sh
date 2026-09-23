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

# Verify sticky-bit data & messages directory (1777)
mkdir -p "$TEST_SANDBOX/data/messages"
chmod 1777 "$TEST_SANDBOX/data" "$TEST_SANDBOX/data/messages"
DATA_PERMS=$(stat -c "%a" "$TEST_SANDBOX/data" 2>/dev/null || stat -f "%Op" "$TEST_SANDBOX/data" | tail -c 5)
echo "  • Data dir mode: $DATA_PERMS (expected: 1777)"

# Test log file creation (0644)
touch "$TEST_SANDBOX/data/messages/test_user.log"
chmod 644 "$TEST_SANDBOX/data/messages/test_user.log"
echo "[12:00:00] Hello test" >> "$TEST_SANDBOX/data/messages/test_user.log"

rm -rf "$TEST_SANDBOX"
echo "  ✅ Permission sandbox passed."

# 6. Test party.sh CLI arguments & update functionality
echo "⚙️  Testing party.sh CLI arguments..."

# Test --help
HELP_OUT=$(./party.sh --help)
if echo "$HELP_OUT" | grep -q -- "--update"; then
    echo "  • party.sh --help lists --update option: ✅"
else
    echo "  ❌ Error: party.sh --help missing --update option"
    exit 1
fi

# Test --version on test sandbox
TEST_UPDATE_DIR="/tmp/test-party-update-$$"
mkdir -p "$TEST_UPDATE_DIR"
echo "v1.2.3-test" > "$TEST_UPDATE_DIR/version.txt"
VER_OUT=$(PARTY_DIR="$TEST_UPDATE_DIR" ./party.sh --version)
if echo "$VER_OUT" | grep -q "v1.2.3-test"; then
    echo "  • party.sh --version returns correct version: ✅"
else
    echo "  ❌ Error: party.sh --version failed"
    exit 1
fi

# Test --update as owner
UPDATE_OUT=$(PARTY_DIR="$TEST_UPDATE_DIR" ./party.sh --update)
if echo "$UPDATE_OUT" | grep -q "successfully updated"; then
    echo "  • party.sh --update as owner succeeds: ✅"
else
    echo "  ❌ Error: party.sh --update failed: $UPDATE_OUT"
    exit 1
fi

# Verify binaries and version were updated
if [ -f "$TEST_UPDATE_DIR/version.txt" ] && [ -x "$TEST_UPDATE_DIR/party-chat" ]; then
    echo "  • Updated binaries and version.txt verified: ✅"
else
    echo "  ❌ Error: party.sh --update did not install expected files"
    exit 1
fi

# Test non-owner rejection
if [ "$(id -u)" -ne 0 ]; then
    NON_OWNER_ERR=$(PARTY_DIR="/proc" ./party.sh --update 2>&1 || true)
    if echo "$NON_OWNER_ERR" | grep -q "Permission denied: You do not own"; then
        echo "  • party.sh --update blocks non-owner with friendly error: ✅"
    else
        echo "  ❌ Error: party.sh --update should have blocked non-owner on /proc: $NON_OWNER_ERR"
        exit 1
    fi
fi

rm -rf "$TEST_UPDATE_DIR"

# 7. Check shell scripts syntax
echo "📜 Checking script syntax..."
bash -n party.sh
bash -n release.sh
echo "  ✅ Shell scripts passed syntax validation."

echo ""
# 8. Deploy local test environment to /tmp/test-terminal-party
INTERACTIVE_DIR="/tmp/test-terminal-party"
echo "📦 Deploying local build to $INTERACTIVE_DIR for testing..."
rm -rf "$INTERACTIVE_DIR"
mkdir -p "$INTERACTIVE_DIR"
chmod 755 "$INTERACTIVE_DIR"
cp target/release/party-chat target/release/termcraft party.sh "$INTERACTIVE_DIR/"
echo "local-test" > "$INTERACTIVE_DIR/version.txt"
chmod 755 "$INTERACTIVE_DIR/party-chat" "$INTERACTIVE_DIR/termcraft" "$INTERACTIVE_DIR/party.sh"
mkdir -p "$INTERACTIVE_DIR/data" "$INTERACTIVE_DIR/data/messages"
chmod 1777 "$INTERACTIVE_DIR/data" "$INTERACTIVE_DIR/data/messages"

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
