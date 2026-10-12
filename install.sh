#!/bin/sh
# Installs the latest release of Drive on a Mac, into the Applications folder.
#
#   curl -fsSL https://raw.githubusercontent.com/necessarylion/drive/main/install.sh | sh
#
# A browser marks every download as "from the internet", and macOS then blocks apps that are not
# signed with an Apple Developer ID. A download with curl does not get that mark, so the app opens.
# Set DRIVE_INSTALL_DIR to install into another folder.
#
# On Linux and Windows, download the installer from the releases page. See the README.
set -eu

REPO="necessarylion/drive"
DEST="${DRIVE_INSTALL_DIR:-/Applications}"
TMP="$(mktemp -d)"
MOUNT="$TMP/mount"
trap 'hdiutil detach "$MOUNT" -quiet 2>/dev/null || true; rm -rf "$TMP"' EXIT

if [ "$(uname -s)" != "Darwin" ]; then
    echo "This script is for macOS. Get the installer for your system from:" >&2
    echo "https://github.com/$REPO/releases/latest" >&2
    exit 1
fi

# The release has one disk image for Apple silicon and one for Intel.
case "$(uname -m)" in
    arm64) ARCH="aarch64" ;;
    *) ARCH="x64" ;;
esac

URL="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
    grep -o "\"browser_download_url\": *\"[^\"]*_${ARCH}\.dmg\"" | head -1 | cut -d'"' -f4)"
if [ -z "$URL" ]; then
    echo "Could not find the latest release of Drive." >&2
    exit 1
fi

echo "Downloading $URL"
curl -fL --progress-bar "$URL" -o "$TMP/Drive.dmg"
mkdir "$MOUNT"
hdiutil attach "$TMP/Drive.dmg" -nobrowse -quiet -mountpoint "$MOUNT"

# Check that the download is the app we expect before it replaces an older copy.
if ! codesign --verify "$MOUNT/Drive.app" 2>/dev/null; then
    echo "The downloaded app is broken. Nothing was installed." >&2
    exit 1
fi

rm -rf "$DEST/Drive.app"
cp -R "$MOUNT/Drive.app" "$DEST/Drive.app"
echo "Drive is installed in $DEST."
echo "To open the application run \"open $DEST/Drive.app\""
