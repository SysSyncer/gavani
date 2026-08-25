#!/bin/sh
# gavani installer — downloads the latest release binary for your platform
# and installs it globally.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/SysSyncer/gavani/main/install.sh | sh
#
# Or download first and review before running:
#   curl -fsSLO https://raw.githubusercontent.com/SysSyncer/gavani/main/install.sh
#   sh install.sh
#
# Installs to /usr/local/bin if possible (with sudo), otherwise ~/.local/bin.

set -eu

REPO="SysSyncer/gavani"
VERSION="${GAVANI_VERSION:-0.1.0}"

# Map OS + architecture to the release asset target name.
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
    Linux*) os_target="unknown-linux-gnu" ;;
    Darwin*) os_target="apple-darwin" ;;
    *)
        echo "error: unsupported OS '$OS' (use scoop/winget on Windows)" >&2
        exit 1
        ;;
esac
case "$ARCH" in
    x86_64|amd64) arch_target="x86_64" ;;
    aarch64|arm64) arch_target="aarch64" ;;
    *)
        echo "error: unsupported architecture '$ARCH'" >&2
        exit 1
        ;;
esac
# Apple Silicon / Intel both use the same archive naming.
TARGET="${arch_target}-${os_target}"

ASSET="gavani-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "==> Downloading ${URL}"
curl -fsSL "$URL" -o "$TMP/$ASSET"

echo "==> Extracting"
tar xzf "$TMP/$ASSET" -C "$TMP"

# Pick an install dir: system-wide when we can sudo, user-local otherwise.
if [ -w /usr/local/bin ] || sudo -n true 2>/dev/null; then
    DEST="/usr/local/bin"
    INSTALL_CMD="sudo install"
else
    DEST="${HOME}/.local/bin"
    mkdir -p "$DEST"
    INSTALL_CMD="install"
    echo "==> No sudo available; installing to $DEST (make sure it is on your PATH)"
fi

$INSTALL_CMD -m755 "$TMP/gavani-${VERSION}/gavani" "$DEST/gavani"

echo "==> Installed gavani ${VERSION} to ${DEST}/gavani"
echo "    Run it with: gavani"
