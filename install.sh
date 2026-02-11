#!/bin/sh
set -e

REPO="filiphric/portmap"
INSTALL_DIR="/usr/local/bin"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
  x86_64)        TARGET="x86_64-apple-darwin" ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

OS=$(uname -s)
if [ "$OS" != "Darwin" ]; then
  echo "Unsupported OS: $OS (only macOS is supported)"
  exit 1
fi

# Get latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$LATEST" ]; then
  echo "Could not determine latest release. Check https://github.com/$REPO/releases"
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$LATEST/portmap-$TARGET.tar.gz"

echo "Downloading portmap $LATEST for $TARGET..."
TMPDIR=$(mktemp -d)
curl -fsSL "$URL" -o "$TMPDIR/portmap.tar.gz"
tar xzf "$TMPDIR/portmap.tar.gz" -C "$TMPDIR"

echo "Installing to $INSTALL_DIR (may require sudo)..."
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMPDIR/portmap" "$INSTALL_DIR/portmap"
else
  sudo mv "$TMPDIR/portmap" "$INSTALL_DIR/portmap"
fi

rm -rf "$TMPDIR"

echo "Installed portmap $LATEST to $INSTALL_DIR/portmap"
echo "Run: sudo portmap"
