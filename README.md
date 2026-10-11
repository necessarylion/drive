<p align="center">
  <img src="Resources/Logo.png" width="128" alt="Drive logo">
</p>

<h1 align="center">Drive</h1>

<p align="center">
  A native macOS app to keep your files and folders in one place, with a backup to your own AWS S3, Google Cloud Storage, Azure or Google Drive.
</p>

![The Drive main window](docs/screenshot.png)

## What it is

Drive is a file manager for one folder on your Mac. It looks and works like a cloud drive app.
Your files stay on your Mac, so the app is fast and works offline.
If you add a bucket, Drive also copies your files there as a backup.

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
- **Cloud backup:** AWS S3, Google Cloud Storage, Azure Blob Storage or Google Drive. New and changed files upload by themselves. You can restore them on another Mac.

## Download

Paste this line in the Terminal app:

```sh
curl -fsSL https://raw.githubusercontent.com/necessarylion/drive/main/install.sh | sh
```

It downloads the latest release and puts `Drive.app` in your Applications folder.
Run the same line again to update.

It works on Apple silicon and Intel Macs with macOS 15 or newer.

### Download by hand

You can also download `Drive-x.y.z.zip` from the
[latest release](https://github.com/necessarylion/drive/releases/latest) and move `Drive.app`
to your Applications folder.

### If macOS does not open the app

Drive is not signed with an Apple Developer ID. When you download it with a browser, macOS
says "Apple could not verify Drive is free of malware" and does not open it. To allow it:

1. Click **Done** in that message.
2. Open **System Settings > Privacy & Security**.
3. Scroll down and click **Open Anyway** next to Drive.

Or run this one time in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Drive.app
```

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

## Backup

The backup is optional. Drive works without it.
Pick one provider in **Drive > Settings** (⌘,): AWS S3, Google Cloud Storage, Azure Blob Storage or Google Drive.

Drive keeps the keys in the Keychain of your Mac. It does not write them to a file.
Each provider has its own keys.

### Set up AWS S3

1. Create an S3 bucket in the AWS console.
2. Create an IAM user with an access key. Give it the policy below.
3. In Drive, open **Drive > Settings** and pick **AWS S3**.
4. Type the bucket name and its region.
5. Paste the access key ID and the secret access key, then click **Save Keys**.
6. Click **Test Connection**.

### Set up Google Cloud Storage

1. Create a bucket in the Google Cloud console.
2. Open **Cloud Storage > Settings > Interoperability** and create an HMAC key.
   The account of the key needs the **Storage Object Admin** role on the bucket.
3. In Drive, open **Drive > Settings** and pick **Google Cloud Storage**.
4. Type the bucket name.
5. Paste the access key and the secret of the HMAC key, then click **Save Keys**.
6. Click **Test Connection**.

### Set up Azure Blob Storage

1. Create a storage account and a container in the Azure portal.
2. Open the container, choose **Shared access tokens**, and make a SAS token.
   Allow Read, Add, Create, Write, Delete and List. Pick an end date far away.
3. In Drive, open **Drive > Settings** and pick **Azure Blob Storage**.
4. Type the storage account name and the container name.
5. Paste the SAS token, then click **Save Keys**.
6. Click **Test Connection**.

The SAS token stops working on its end date. Make a new one then and save it again.

### Set up Google Drive

Google only lets a registered app sign in. So you make a free client ID first. You do this one time.

1. Open the [Google Cloud console](https://console.cloud.google.com) and create a project.
2. Open **APIs & Services > Library**, find **Google Drive API**, and click **Enable**.
3. Open **Google Auth Platform** and click **Get started**. Type an app name and your email.
   Pick **External** as the audience. Do not add a logo.
4. Open **Data Access**, click **Add or remove scopes**, and add `.../auth/drive.file`.
5. Open **Audience** and click **Publish app**. Without this, you must sign in again every 7 days.
6. Open **Clients** and click **Create client**. Pick **iOS** as the application type.
   Type `com.zinkyaw.drive` as the bundle ID. Copy the client ID.
7. In Drive, open **Drive > Settings** and pick **Google Drive**.
8. Paste the client ID. Type a name for the backup folder, like `Drive Backup`.
9. Click **Sign In with Google…** and allow the access.
10. Click **Test Connection**.

Drive can only see the files it uploaded itself. It cannot see your other Google Drive files.
Google does not review an app that only asks for this access.

### IAM policy for AWS

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
- The automatic backup only uploads. It never removes anything from the backup.
- **Backup > Back Up Now** also looks for files that are in the backup but not on your Mac.
  This happens after a rename or a delete. Drive asks you before it removes them from the backup.
- **Backup > Restore from Backup** downloads the files that are in the backup but not on your Mac.
  It never replaces a file on your Mac.
- The pause button in the status bar stops the automatic backup.

### Limits

- One file can be at most 5 GB.
- Bucket names with a dot or “_” in them are not supported.
- Stars, pins, folder colors, the trash and empty folders are not backed up.
- Drive is a backup tool, not a sync tool. It does not merge changes from two Macs.
- On Google Drive, a removed file goes to the trash of Google Drive. Google deletes it after 30 days.
- Turn on versioning for your bucket if you want to undo a removed or replaced file.

## Project layout

The code follows one direction: view → controller → service → repository → model.

```
Sources/Drive/
  App/           The app entry point. All parts are wired together here.
  Config/        Where data is stored, and the backup settings.
  Constants/     Global constants and the colors.
  Models/        Plain data types.
  Repositories/  All access to the disk, the Keychain, S3, Azure and Google Drive.
  Services/      The rules of the drive, the backup, and AWS request signing.
  Controllers/   The state of the windows.
  Views/         SwiftUI views.
Tests/DriveTests/  Tests for the services and the backup code.
Scripts/           The script that builds the app icon.
Resources/         Info.plist, the logo and the app icon.
```

Requests to S3 are signed in `Services/AWSSigner.swift` with AWS Signature Version 4.
The tests check the signer against the examples in the AWS documentation.
Google Cloud Storage takes the same signed requests, so `S3Repository` is used for it too.
Azure has its own calls in `Repositories/AzureRepository.swift`.
Google Drive has its calls in `Repositories/GoogleDriveRepository.swift`, and its sign-in in `GoogleAuthRepository.swift`.

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
