# Installs the latest release of Drive on Windows (x64, ARM and 32-bit).
#
#   irm https://raw.githubusercontent.com/necessarylion/drive/main/install.ps1 | iex
#
# It downloads the installer for the chip of this computer and runs it without questions.
# Run the same line again to update.
# Set $env:DRIVE_VERSION to a tag, like v1.2.0, to install that release and not the latest one.
#
# On macOS and Linux, use install.sh. See the README.
$ErrorActionPreference = 'Stop'
# The progress bar of Windows PowerShell 5 makes a download many times slower.
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5 on an older system does not use TLS 1.2 by itself, and GitHub needs it.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$repo = 'necessarylion/drive'

# A 32-bit PowerShell on a 64-bit Windows says "x86". The real chip is then in PROCESSOR_ARCHITEW6432.
$chip = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
switch ($chip) {
    'AMD64' { $arch = 'x64' }
    'ARM64' { $arch = 'arm64' }
    'x86' { $arch = 'x86' }
    default { throw "Drive has no build for this chip ($chip)." }
}

$api = "https://api.github.com/repos/$repo/releases/latest"
if ($env:DRIVE_VERSION) {
    $api = "https://api.github.com/repos/$repo/releases/tags/$env:DRIVE_VERSION"
}
$release = Invoke-RestMethod -Uri $api -UseBasicParsing
$asset = $release.assets | Where-Object { $_.name -like "*_$arch-setup.exe" } | Select-Object -First 1
if (-not $asset) {
    throw "This release of Drive has no installer for your system. See https://github.com/$repo/releases/latest"
}

$installer = Join-Path $env:TEMP $asset.name
Write-Host "Downloading $($asset.browser_download_url)"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installer -UseBasicParsing

try {
    # /S runs the installer without its window. It installs for this user, so no admin rights are needed.
    $setup = Start-Process -FilePath $installer -ArgumentList '/S' -Wait -PassThru
    if ($setup.ExitCode -ne 0) {
        throw "The installer failed with code $($setup.ExitCode). Nothing was installed."
    }
}
finally {
    Remove-Item $installer -ErrorAction SilentlyContinue
}
Write-Host 'Drive is installed. Open it from the Start menu.'
