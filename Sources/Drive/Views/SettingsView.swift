import AuthenticationServices
import SwiftUI

struct SettingsView: View {
    @Environment(BackupController.self) private var backup
    @Environment(\.webAuthenticationSession) private var webAuthenticationSession
    @State private var accessKey = ""
    @State private var secretKey = ""
    @AppStorage(AppConstants.storageRootKey) private var storageRoot = ""
    @AppStorage(AppConstants.backupProviderKey) private var provider = BackupProvider.s3
    @AppStorage(AppConstants.azureAccountKey) private var account = ""
    @AppStorage(AppConstants.googleClientIDKey) private var clientID = ""
    @AppStorage(AppConstants.awsBucketKey) private var bucket = ""
    @AppStorage(AppConstants.awsRegionKey) private var region = AppConstants.awsDefaultRegion
    @AppStorage(AppConstants.awsPrefixKey) private var prefix = ""

    var body: some View {
        Form {
            Section {
                LabeledContent("Folder") {
                    Text(storageRoot.isEmpty ? AppConfig.defaultStorageRoot.path : storageRoot)
                        .lineLimit(2)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                }
                HStack {
                    Spacer()
                    Button("Use Default", action: useDefaultFolder)
                        .disabled(storageRoot.isEmpty)
                    Button("Choose…", action: chooseFolder)
                }
            } header: {
                Text("Storage")
            } footer: {
                Text(
                    "This is where Drive keeps your files. A change takes effect after you quit and open Drive again. "
                        + "Files in the old folder stay there. Drive does not move them."
                )
                .font(.caption)
                .foregroundStyle(.secondary)
            }

            Section {
                Picker("Provider", selection: $provider) {
                    ForEach(BackupProvider.allCases) { provider in
                        Text(provider.title)
                            .tag(provider)
                    }
                }
                .onChange(of: provider, backup.refreshKeys)
                if provider == .azure {
                    TextField("Storage account", text: $account, prompt: Text("mydriveaccount"))
                }
                if provider == .googleDrive {
                    TextField("Client ID", text: $clientID, prompt: Text("123-abc.apps.googleusercontent.com"))
                }
                TextField(bucketLabel, text: $bucket, prompt: Text("my-drive-backup"))
                if provider == .s3 {
                    TextField("Region", text: $region, prompt: Text(AppConstants.awsDefaultRegion))
                }
                TextField("Folder in it", text: $prefix, prompt: Text("Optional"))
            } header: {
                Text("Backup")
            } footer: {
                Text("Drive uploads new and changed files by itself when a bucket, container or folder is set.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section {
                if let hint = backup.savedKeyHint {
                    LabeledContent("Saved key") {
                        Text(hint)
                            .monospaced()
                    }
                }
                if provider == .googleDrive {
                    // Google Drive has no keys to type. The user signs in with the button below.
                } else if provider == .azure {
                    SecureField("SAS token", text: $secretKey, prompt: Text("Paste it here"))
                } else {
                    TextField("Access key ID", text: $accessKey, prompt: Text(provider == .s3 ? "AKIA…" : "GOOG…"))
                    SecureField("Secret access key", text: $secretKey, prompt: Text("Paste it here"))
                }
                HStack {
                    Text(backup.connectionResult)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                    Spacer()
                    if backup.isTesting {
                        ProgressView()
                            .controlSize(.small)
                    }
                    Button("Test Connection", action: backup.testConnection)
                        .disabled(backup.isTesting || bucket.isEmpty)
                    Button("Remove", role: .destructive, action: backup.removeKeys)
                        .disabled(backup.savedKeyHint == nil)
                    if provider == .googleDrive {
                        Button("Sign In with Google…", action: signIn)
                            .disabled(clientID.isEmpty || bucket.isEmpty)
                    } else {
                        Button("Save Keys", action: saveKeys)
                            .disabled(secretKey.isEmpty || (accessKey.isEmpty && provider != .azure))
                    }
                }
            } header: {
                Text(provider == .googleDrive ? "Google Account" : "Access Keys")
            } footer: {
                Text(keysHelp + " The keys are kept in the Keychain of this Mac.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: 540)
        .fixedSize(horizontal: false, vertical: true)
    }

    private var keysHelp: String {
        switch provider {
        case .s3: "Use the access key of an IAM user."
        case .gcs: "Use an HMAC key. Make one in Cloud Storage > Settings > Interoperability."
        case .azure: "Make a SAS token for the container. Allow Read, Add, Create, Write, Delete and List."
        case .googleDrive: "Drive can only see the files it uploaded itself, not your other Google Drive files."
        }
    }

    private var bucketLabel: String {
        switch provider {
        case .s3, .gcs: "Bucket"
        case .azure: "Container"
        case .googleDrive: "Folder in Google Drive"
        }
    }

    private func signIn() {
        Task { await runSignIn() }
    }

    private func runSignIn() async {
        guard let url = backup.googleSignInURL() else { return }
        // This throws when the user closes the sign-in window. Then there is nothing to do.
        guard
            let callback = try? await webAuthenticationSession.authenticate(
                using: url, callbackURLScheme: backup.googleRedirectScheme)
        else { return }
        await backup.finishGoogleSignIn(callback)
    }

    private func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.prompt = "Use This Folder"
        if panel.runModal() == .OK, let url = panel.url {
            storageRoot = url.path
        }
    }

    private func saveKeys() {
        if backup.saveKeys(accessKey: accessKey, secretKey: secretKey) {
            // Do not keep the secret on screen after it is saved.
            accessKey = ""
            secretKey = ""
        }
    }

    private func useDefaultFolder() {
        storageRoot = ""
    }
}
