#!/bin/sh
# Installs the latest release of Drive on macOS or Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/necessarylion/drive/main/install.sh | sh
#
# macOS (Apple silicon and Intel): puts Drive.app in the Applications folder.
#   A browser marks every download as "from the internet", and macOS then blocks apps that are not
#   signed with an Apple Developer ID. A download with curl does not get that mark, so the app opens.
# Linux (x64 and arm64): installs the .deb or the .rpm with the package manager of the system.
#   A system with no such package manager gets the AppImage in ~/.local/bin.
# Windows: use install.ps1. See the README.
#
# Set DRIVE_INSTALL_DIR to install the app (macOS) or the AppImage (Linux) into another folder.
# Set DRIVE_VERSION to a tag, like v1.2.0, to install that release and not the latest one.
set -eu

REPO="necessarylion/drive"
PAGE="https://github.com/$REPO/releases/latest"
TMP="$(mktemp -d)"
MOUNT="$TMP/mount"
trap 'hdiutil detach "$MOUNT" -quiet 2>/dev/null || true; rm -rf "$TMP"' EXIT

fail() {
    echo "$1" >&2
    exit 1
}

# Runs a command as root. Asks for the password with sudo when the user is not root.
as_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    else
        sudo "$@"
    fi
}

# Downloads the file of the release whose name ends with $1 (a pattern for grep) to $2.
download() {
    url="$(grep -o "\"browser_download_url\": *\"[^\"]*$1\"" "$TMP/release.json" | head -1 | cut -d'"' -f4)"
    [ -n "$url" ] || fail "This release of Drive has no file for your system. See $PAGE"
    echo "Downloading $url"
    curl -fL --progress-bar "$url" -o "$2"
}

install_mac() {
    dest="${DRIVE_INSTALL_DIR:-/Applications}"
    # The release has one disk image for Apple silicon and one for Intel.
    case "$(uname -m)" in
        arm64) download '_aarch64\.dmg' "$TMP/Drive.dmg" ;;
        *) download '_x64\.dmg' "$TMP/Drive.dmg" ;;
    esac
    mkdir "$MOUNT"
    hdiutil attach "$TMP/Drive.dmg" -nobrowse -quiet -mountpoint "$MOUNT"

    # Check that the download is the app we expect before it replaces an older copy.
    codesign --verify "$MOUNT/Drive.app" 2>/dev/null || fail "The downloaded app is broken. Nothing was installed."

    rm -rf "$dest/Drive.app"
    cp -R "$MOUNT/Drive.app" "$dest/Drive.app"
    echo "Drive is installed in $dest."
    echo "To open the application run \"open $dest/Drive.app\""
}

install_linux() {
    # The same chip has another name in each kind of file.
    case "$(uname -m)" in
        x86_64 | amd64) deb="amd64" rpm="x86_64" image="amd64" ;;
        aarch64 | arm64) deb="arm64" rpm="aarch64" image="aarch64" ;;
        *) fail "Drive has no build for this chip ($(uname -m)). It needs 64-bit Intel, AMD or ARM." ;;
    esac

    if command -v apt-get >/dev/null 2>&1; then
        download "_$deb\\.deb" "$TMP/drive.deb"
        # Everyone must be able to read the file, because apt reads it as another user.
        chmod 755 "$TMP" && chmod 644 "$TMP/drive.deb"
        as_root apt-get install -y "$TMP/drive.deb"
    elif command -v dnf >/dev/null 2>&1; then
        download "\\.$rpm\\.rpm" "$TMP/drive.rpm"
        as_root dnf install -y "$TMP/drive.rpm"
    elif command -v yum >/dev/null 2>&1; then
        download "\\.$rpm\\.rpm" "$TMP/drive.rpm"
        as_root yum install -y "$TMP/drive.rpm"
    elif command -v zypper >/dev/null 2>&1; then
        download "\\.$rpm\\.rpm" "$TMP/drive.rpm"
        # The package is not signed, so zypper must be told that this is fine.
        as_root zypper --non-interactive install --allow-unsigned-rpm "$TMP/drive.rpm"
    else
        dest="${DRIVE_INSTALL_DIR:-$HOME/.local/bin}"
        download "_$image\\.AppImage" "$TMP/Drive.AppImage"
        mkdir -p "$dest"
        mv "$TMP/Drive.AppImage" "$dest/Drive.AppImage"
        chmod +x "$dest/Drive.AppImage"
        echo "Drive is installed. To open the application run \"$dest/Drive.AppImage\""
        return
    fi
    echo "Drive is installed. Open it from the list of your applications."
}

if [ -n "${DRIVE_VERSION:-}" ]; then
    API="https://api.github.com/repos/$REPO/releases/tags/$DRIVE_VERSION"
else
    API="https://api.github.com/repos/$REPO/releases/latest"
fi
curl -fsSL "$API" -o "$TMP/release.json" || fail "Could not find the release of Drive. See $PAGE"

case "$(uname -s)" in
    Darwin) install_mac ;;
    Linux) install_linux ;;
    *) fail "This script is for macOS and Linux. On Windows, use install.ps1. See $PAGE" ;;
esac
