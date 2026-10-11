#!/bin/sh
# Installs the latest release of Drive into the Applications folder.
#
#   curl -fsSL https://raw.githubusercontent.com/necessarylion/drive/main/install.sh | sh
#
# A browser marks every download as "from the internet", and macOS then blocks apps that are not
# signed with an Apple Developer ID. A download with curl does not get that mark, so the app opens.
# Set DRIVE_INSTALL_DIR to install into another folder.
set -eu

REPO="necessarylion/drive"
DEST="${DRIVE_INSTALL_DIR:-/Applications}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

URL="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
    grep -o '"browser_download_url": *"[^"]*\.zip"' | head -1 | cut -d'"' -f4)"
if [ -z "$URL" ]; then
    echo "Could not find the latest release of Drive." >&2
    exit 1
fi

echo "Downloading $URL"
curl -fL --progress-bar "$URL" -o "$TMP/Drive.zip"
ditto -x -k "$TMP/Drive.zip" "$TMP"

# Check that the download is the app we expect before it replaces an older copy.
if ! codesign --verify "$TMP/Drive.app" 2>/dev/null; then
    echo "The downloaded app is broken. Nothing was installed." >&2
    exit 1
fi

rm -rf "$DEST/Drive.app"
mv "$TMP/Drive.app" "$DEST/Drive.app"
echo "Drive is installed in $DEST. Open it from there."
