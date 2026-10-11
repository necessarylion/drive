<p align="center">
  <img src="Resources/Logo.png" width="128" alt="Drive logo">
</p>

<h1 align="center">Drive</h1>

<p align="center">
  A native macOS app to keep your files and folders in one place, with a backup to your own AWS S3 bucket.
</p>

![The Drive main window](docs/screenshot.png)

## What it is

Drive is a file manager for one folder on your Mac. It looks and works like a cloud drive app.
Your files stay on your Mac, so the app is fast and works offline.
If you add an S3 bucket, Drive also copies your files there as a backup.

Drive is written in Swift and SwiftUI. It has no third-party dependencies.

## Features

- **Three views:** list, grid with file previews, and columns.
- **Folders:** make folders, give them a color, and pin them to the sidebar.
- **Add files:** use the New button, or drag files and folders in from Finder.
- **Organize:** rename, duplicate, move, star, and drag items onto folders.
- **Find:** search by name, sort by name, date or size, and see recent files.
- **Trash:** put items back, or delete them forever. Drive asks before it deletes.
- **Details pane:** preview, type, size, dates and backup state of the selected item.
- **Viewer window:** open a file in its own window. A PDF gets page thumbnails, zoom, search and print.
- **Quick Look:** preview a file with ⌘Y.
- **Menu bar icon:** see the backup state and your recent files without opening the window.
- **AWS S3 backup:** new and changed files upload by themselves. You can restore them on another Mac.

## Download

1. Open the [latest release](https://github.com/necessarylion/drive/releases/latest).
2. Download `Drive-x.y.z.zip` and open it.
3. Move `Drive.app` to your Applications folder.

The app is not signed with an Apple Developer ID, so macOS blocks it the first time.
To allow it:

1. Open `Drive.app` once. macOS says it cannot open the app.
2. Open **System Settings > Privacy & Security**.
3. Scroll down and click **Open Anyway** next to Drive.

Or run this one time in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Drive.app
```

The download works on Apple silicon and Intel Macs with macOS 15 or newer.

## Build from source

### Requirements

- macOS 15 or newer
- Xcode 26 or newer (Swift 6.2)

### Build and run

```sh
git clone https://github.com/necessarylion/drive.git
cd drive
make run
```

`make run` builds the app, puts it in `build/Drive.app`, and opens it.

| Command       | What it does                                         |
| ------------- | ---------------------------------------------------- |
| `make run`    | Build the app and open it                            |
| `make app`    | Build `build/Drive.app` without opening it           |
| `make test`   | Run the tests                                        |
| `make format` | Format the code with `swift format`                  |
| `make lint`   | Check the code style with `swift format lint`        |
| `make release VERSION=1.2.0` | Build a zip for download, for both Apple silicon and Intel |
| `make icon`   | Rebuild the app icon from `Resources/Logo.png`       |

### Make a release

Push a tag that starts with `v`:

```sh
git tag v1.2.0
git push origin v1.2.0
```

GitHub Actions then builds the app and adds `Drive-1.2.0.zip` to the release of that tag.
You can also make the release and the tag on the GitHub website. The zip is added to it.

The app is signed ad hoc. To avoid the macOS warning for your users, sign it with an
Apple Developer ID and notarize it.

## Where your files are

By default, Drive keeps everything in `~/Library/Application Support/Drive`:

| Path            | What is in it                                       |
| --------------- | --------------------------------------------------- |
| `Files/`        | Your files and folders, as normal files on disk     |
| `Trash/`        | Items you moved to the trash                        |
| `metadata.json` | Stars, pinned folders, folder colors, trash origins |

You can pick another folder in **Drive > Settings > Storage**.
The change takes effect after you quit and open Drive again.
Drive does not move files from the old folder to the new one.

## AWS S3 backup

The backup is optional. Drive works without it.

### Set it up

1. Create an S3 bucket in the AWS console.
2. Create an IAM user with an access key. Give it the policy below.
3. In Drive, open **Drive > Settings** (⌘,).
4. Type the bucket name and its region.
5. Paste the access key ID and the secret access key, then click **Save Keys**.
6. Click **Test Connection**.

Drive keeps the keys in the Keychain of your Mac. It does not write them to a file.

### IAM policy

Replace `my-drive-backup` with the name of your bucket.

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "s3:ListBucket",
      "Resource": "arn:aws:s3:::my-drive-backup"
    },
    {
      "Effect": "Allow",
      "Action": ["s3:PutObject", "s3:GetObject", "s3:DeleteObject"],
      "Resource": "arn:aws:s3:::my-drive-backup/*"
    }
  ]
}
```

### How it works

- Drive uploads a file when it is new, or when the copy on your Mac is newer or has another size.
- The automatic backup only uploads. It never removes anything from S3.
- **Backup > Back Up Now** also looks for files that are in S3 but not on your Mac.
  This happens after a rename or a delete. Drive asks you before it removes them from S3.
- **Backup > Restore from AWS** downloads the files that are in S3 but not on your Mac.
  It never replaces a file on your Mac.
- The pause button in the status bar stops the automatic backup.

### Limits

- One file can be at most 5 GB.
- Bucket names with a dot in them are not supported.
- Stars, pins, folder colors, the trash and empty folders are not backed up.
- Drive is a backup tool, not a sync tool. It does not merge changes from two Macs.
- Turn on versioning for your bucket if you want to undo a removed or replaced file.

## Project layout

The code follows one direction: view → controller → service → repository → model.

```
Sources/Drive/
  App/           The app entry point. All parts are wired together here.
  Config/        Where data is stored, and the AWS settings.
  Constants/     Global constants and the colors.
  Models/        Plain data types.
  Repositories/  All access to the disk, the Keychain and S3.
  Services/      The rules of the drive, the backup, and AWS request signing.
  Controllers/   The state of the windows.
  Views/         SwiftUI views.
Tests/DriveTests/  Tests for the services and the AWS code.
Scripts/           The script that builds the app icon.
Resources/         Info.plist, the logo and the app icon.
```

Requests to S3 are signed in `Services/AWSSigner.swift` with AWS Signature Version 4.
The tests check the signer against the examples in the AWS documentation.

## Development

To try the app without touching your real files, point it at another folder:

```sh
make app
open --env DRIVE_STORAGE_ROOT=/tmp/drive-demo build/Drive.app
```

You can also open `Package.swift` in Xcode.

## Contributing

Issues and pull requests are welcome.

Before you open a pull request, please run:

```sh
make format
make lint
make test
```

## License

Drive is released under the MIT License. See [LICENSE](LICENSE).
