# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Drive is a native macOS app (Swift 6.2, SwiftUI, macOS 15+). It manages the files in one folder on the Mac and can back them up to an AWS S3 bucket. It is a Swift package with one executable target. It has no third-party dependencies. Keep it that way: S3 calls and AWS Signature Version 4 signing are written by hand on top of `URLSession` and `CryptoKit`.

## Commands

| Command | What it does |
| --- | --- |
| `make run` | Build the release binary, wrap it in `build/Drive.app`, and open it |
| `make app` | The same, without opening it |
| `make test` | Run all tests (`swift test`) |
| `make format` | Format with `swift format` |
| `make lint` | Check the style with `swift format lint --strict` |
| `make release VERSION=1.2.0` | Build a universal zip (arm64 + x86_64) |
| `make icon` | Rebuild `Resources/AppIcon.icns` from `Resources/Logo.png` |

Run one test or one test file (Swift Testing, not XCTest):

```sh
swift test --filter DriveServiceTests
swift test --filter DriveServiceTests/fullLifeOfAFile
```

Run the app without touching the real files:

```sh
make app
open --env DRIVE_STORAGE_ROOT=/tmp/drive-demo build/Drive.app
```

Run `make format`, `make lint` and `make test` before a pull request. The style is in `.swift-format`: 120 columns, 4 spaces.

A release is made by pushing a tag that starts with `v`. `.github/workflows/release.yml` then runs `make release` and adds the zip to the GitHub release. The app is only signed ad hoc.

## Architecture

The code goes in one direction: view → controller → service → repository → model. Each layer has its own folder in `Sources/Drive/`.

- **`App/DriveApp.swift`**: `AppContainer` is the one place where all parts are built and wired together (dependency injection by `init`). Add new services and repositories there. The app has four scenes: the main window, the viewer windows, the menu bar icon, and Settings. The two controllers are passed to the views with `.environment(...)`.
- **Controllers** (`@Observable` classes): hold the state of the windows and turn user actions into service calls. `DriveController` owns navigation, selection, dialogs and the item lists. `BackupController` owns the backup state.
- **Services**: the rules. `DriveService` (list, add, rename, star, pin, trash), `BackupService` (compare with S3, upload, restore), `AWSSigner` (SigV4).
- **Repositories**: all outside access. `FileRepository` (disk), `MetadataRepository` (the JSON file), `KeychainRepository` (AWS keys), `S3Repository` (network).
- **Config / Constants**: `AppConfig` (where data is stored), `AWSConfig` (bucket, region, prefix from `UserDefaults`), `AppConstants` (all keys, limits and sizes), `Theme` (colors).

### Things to know before you change code

- **Everything is on the main actor by default.** `Package.swift` sets `.defaultIsolation(MainActor.self)`. Plain data types are marked `nonisolated` and `Sendable`. Work that must leave the main thread is marked `@concurrent nonisolated` (see `FileRepository.copy`).
- **Files are normal files on disk.** The storage root has `Files/`, `Trash/`, `metadata.json` and `backup-state.json`. The root comes from `DRIVE_STORAGE_ROOT`, then the folder saved in Settings, then `~/Library/Application Support/Drive`.
- **Metadata is keyed by full path.** Stars, pins and folder colors are stored by path in `DriveMetadata`. After any move or rename, call `metadata.repath(from:to:)`, or the item loses them. Every field of `DriveMetadata` is decoded with `decodeIfPresent`, so a file from an older version still loads. Do the same for new fields.
- **Paths must be canonical.** `FileRepository.preparedFolder` returns the real path (`/private/var`, not `/var`), and `FileRepository.item(at:)` builds a URL without a trailing slash. Path checks use `hasPrefix` on these, so build URLs with `directoryHint: .notDirectory`.
- **The trash uses slots.** A trashed item moves to `Trash/<UUID>/<name>`. `metadata.trashOrigins` maps the slot id to the old path. `DriveService.trashSlot(of:)` is the guard that stops restore and delete from touching anything outside the trash.
- **How the automatic backup starts.** `DriveController` adds 1 to `changeCount` after every change to the files. `ContentView` watches it and calls `BackupController.autoBackUp`. A new action that changes files must go through the same path in `DriveController`.
- **Backup rules.** `BackupService.plan` only compares; `run` does the work. A file uploads when its size differs or the local copy is newer (the same rule as `aws s3 sync`). The automatic backup never removes anything from S3. Only the manual "Back Up Now" can, and it asks first. Restore never replaces a local file, and it refuses keys with `..` in them.
- **There are four backup providers.** `BackupProvider` names them: AWS S3, Google Cloud Storage, Azure Blob Storage and Google Drive. Each one is a `BackupRepository` (list, upload, download, delete), and `BackupService.repository` picks one. `S3Repository` also serves Google Cloud Storage. `AzureRepository` signs in with a SAS token. `GoogleDriveRepository` has no paths, only file IDs, so its `list` builds the paths and the other calls use the IDs from the last `list`. `GoogleAuthRepository` does the Google sign-in (OAuth 2 with PKCE, scope `drive.file`).
- **Backup settings wait for Save.** The Settings window edits a draft `AWSConfig`. Only `BackupController.saveSettings` makes it the one in use. Each provider has its own bucket and folder in `UserDefaults` (`AppConstants.bucketKey`).
- **The backup state is saved.** `BackupService` keeps which files are in the backup in `backup-state.json`, so the icons are right at the start. The state is for one destination. `BackupService.follow` drops it when the destination changes.
- **AWS keys live only in the Keychain.** The bucket, region and prefix are in `UserDefaults`. `AWSConfig.validate` limits the bucket and region to safe letters because they become part of the host name.
- **`// ponytail:` comments** mark a simple choice made on purpose, with its limit and how to grow it (for example one `PUT` per file, so 5 GB is the biggest file). Read them before you "fix" such code.

### Tests

Tests are in `Tests/DriveTests/` and use Swift Testing (`@Test`, `#expect`). `DriveServiceTests` runs the real `DriveService` and `FileRepository` on a new temp folder for each test. `AWSTests` checks the signer against the examples in the AWS documentation. There are no tests for views or controllers.
