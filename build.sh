#!/usr/bin/env bash
set -euo pipefail

DEPS=(
    build-essential
    curl
    wget
    libssl-dev
    libgtk-3-dev
    libwebkit2gtk-4.1-dev
    libjavascriptcoregtk-4.1-dev
    libayatana-appindicator3-dev
    librsvg2-dev
    patchelf
    pkg-config
)

TAURI_CLI_VERSION="2.11.4"

echo "[1/4] Installing system dependencies (requires sudo)..."
sudo apt-get update
sudo apt-get install -y "${DEPS[@]}"

if ! command -v rustc &> /dev/null; then
    echo "[2/4] Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "[2/4] Rust already installed."
fi

echo "[3/4] Installing Tauri CLI ${TAURI_CLI_VERSION}..."
cargo install tauri-cli --version "$TAURI_CLI_VERSION" --locked

echo "[4/4] Building release bundles..."
cd "$(dirname "$0")"
cargo tauri build

echo "Done. Bundles:"
echo "  src-tauri/target/release/bundle/deb/*.deb"
echo "  src-tauri/target/release/bundle/appimage/*.AppImage"
