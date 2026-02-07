#!/usr/bin/env bash
set -euo pipefail

# ── Texnouz OCPP — Build & Bundle Script ─────────────────────
# Builds ocpp-service + ocpp-desktop and creates installer packages
#
# Usage:
#   ./build.sh              — build all (debug)
#   ./build.sh release      — build release + bundle (deb, rpm, appimage)
#   ./build.sh release deb  — build release + specific bundle target

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

MODE="${1:-debug}"
BUNDLE_TARGET="${2:-}"

TARGET_TRIPLE=$(rustc -vV | grep host | awk '{print $2}')
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Texnouz OCPP — Build & Bundle                      ║"
echo "╠══════════════════════════════════════════════════════╣"
echo "║  Target: $TARGET_TRIPLE"
echo "║  Mode:   $MODE"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# ── Step 1: Build ocpp-service ──────────────────────────────
echo "📦 [1/3] Building ocpp-service..."
if [ "$MODE" = "release" ]; then
    cargo build --release --bin ocpp-service
    SERVICE_BIN="target/release/ocpp-service"
else
    cargo build --bin ocpp-service
    SERVICE_BIN="target/debug/ocpp-service"
fi
echo "   ✅ ocpp-service built: $SERVICE_BIN"

# ── Step 2: Copy binary for Tauri externalBin ───────────────
echo "📋 [2/3] Preparing external binary for Tauri..."
mkdir -p desktop/binaries
cp "$SERVICE_BIN" "desktop/binaries/ocpp-service-${TARGET_TRIPLE}"
echo "   ✅ Copied to desktop/binaries/ocpp-service-${TARGET_TRIPLE}"

# ── Step 3: Build desktop app / bundle ──────────────────────
if [ "$MODE" = "release" ]; then
    echo "🏗️  [3/3] Building Tauri release bundle..."

    if [ -n "$BUNDLE_TARGET" ]; then
        cargo tauri build --bundles "$BUNDLE_TARGET"
    else
        cargo tauri build
    fi

    echo ""
    echo "════════════════════════════════════════════════════════"
    echo "  ✅ Build complete! Packages are in:"
    echo "     target/release/bundle/"
    echo ""
    ls -la target/release/bundle/*/ 2>/dev/null || true
    echo "════════════════════════════════════════════════════════"
else
    echo "🔨 [3/3] Building Tauri desktop (debug)..."
    cargo build -p ocpp-desktop
    echo "   ✅ Debug build complete: target/debug/ocpp-desktop"
fi
