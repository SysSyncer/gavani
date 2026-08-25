#!/usr/bin/env sh
# gavani installer – auto‑detects OS/arch, fetches the latest release,
# and installs the binary globally (sudo) or user‑local (no sudo).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/SysSyncer/gavani/main/install.sh | sh
#
# Or download first and review before running:
#   curl -fsSLO https://raw.githubusercontent.com/SysSyncer/gavani/main/install.sh
#   sh install.sh

set -eu

REPO="SysSyncer/gavani"

# ---- 1. detect platform -------------------------------------------------
UNAME_S=$(uname -s)
UNAME_M=$(uname -m)

case "$UNAME_S" in
    Linux*)   OS="linux"   ;;
    Darwin*)  OS="macos"   ;;
    *)        echo "error: unsupported OS" >&2; exit 1 ;;
esac

case "$UNAME_M" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)            echo "error: unsupported arch" >&2; exit 1 ;;
esac

# ---- 2. fetch latest version & asset map -------------------------------
API="https://api.github.com/repos/${REPO}/releases/latest"
# NOTE: jq must be available; if not, we fall back to the hardcoded version.
VERSION=$(curl -fsSL "${API}" | jq -r .tag_name | sed 's/^v//') || {
    # fallback if jq missing or API fails
    VERSION="0.1.0"
}

# Map detected platform to the asset name(s) published in the release.
case "${OS}-${ARCH}" in
    linux-x86_64)   ASSET="gavani-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" ;;
    macos-x86_64)   ASSET="gavani-${VERSION}-x86_64-apple-darwin.tar.gz" ;;
    macos-aarch64)  ASSET="gavani-${VERSION}-aarch64-apple-darwin.tar.gz" ;;
    linux-aarch64)  ASSET="gavani-${VERSION}-aarch64-unknown-linux-gnu.tar.gz" ;;
    # Windows assets are published separately; add them if needed.
    *)              echo "error: no asset for ${OS}-${ARCH}" >&2; exit 1 ;;
esac

ASSET_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"

# ---- 3. choose install directory ----------------------------------------
if sudo -n true 2>/dev/null; then
    DEST="/usr/local/bin"
    INSTALL="sudo install -m755"
else
    DEST="${HOME}/.local/bin"
    mkdir -p "$DEST"
    INSTALL="install -m755"
    echo "No sudo available - installing to $DEST (make sure it is on your PATH)"
fi

# ---- 4. download + install ---------------------------------------------
echo "Downloading gvana ${VERSION} for ${OS}-${ARCH}"
curl -fsSL "${ASSET_URL}" -o "/tmp/gavani-${VERSION}.tar.gz"

echo "Extracting"
tar xzf "/tmp/gavani-${VERSION}.tar.gz" -C /tmp

echo "Installing to ${DEST}"
$INSTALL "/tmp/gavani-${VERSION}/gavani" "$DEST/gavani"

echo "Installed gvana ${VERSION} to ${DEST}/gavana"
echo "Run it with: gvana"