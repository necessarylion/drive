import AuthenticationServices
import SwiftUI

struct SettingsView: View {
    @Environment(BackupController.self) private var backup
    @Environment(\.webAuthenticationSession) private var webAuthenticationSession
    @State private var accessKey = ""
    @State private var secretKey = ""
    @AppStorage(AppConstants.storageRootKey) private var storageRoot = ""
    /// The backup settings on screen. The backup does not use them until the user chooses Save.
    @State private var draft = AWSConfig.saved()
    /// The backup settings that are in use now.
    @State private var saved = AWSConfig.saved()

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
                Picker("Provider", selection: $draft.provider) {
                    ForEach(BackupProvider.allCases) { provider in
                        Text(provider.title)
                            .tag(provider)
                    }
                }
                .onChange(of: draft.provider, providerChanged)
                if provider == .azure {
                    TextField("Storage account", text: $draft.account, prompt: Text("mydriveaccount"))
                }
                if provider == .googleDrive {
                    TextField("Client ID", text: $draft.clientID, prompt: Text("123-abc.apps.googleusercontent.com"))
                }
                TextField(bucketLabel, text: $draft.bucket, prompt: Text("my-drive-backup"))
                if provider == .s3 {
                    TextField("Region", text: $draft.region, prompt: Text(AppConstants.awsDefaultRegion))
                }
                TextField("Folder in it", text: $draft.prefix, prompt: Text("Optional"))
                HStack {
                    Text(draft == saved ? "" : "These changes are not saved yet.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("Revert", action: revert)
                        .disabled(draft == saved)
                    Button("Save", action: save)
                        .disabled(draft == saved)
                }
            } header: {
                Text("Backup")
            } footer: {
                Text(
                    "Drive uploads new and changed files by itself when a bucket, container or folder is set. "
                        + "A change here takes effect after you choose Save."
                )
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
                    Button("Test Connection", action: testConnection)
                        .disabled(backup.isTesting || draft.bucket.isEmpty)
                    Button("Remove", role: .destructive, action: removeKeys)
                        .disabled(backup.savedKeyHint == nil)
                    if provider == .googleDrive {
                        Button("Sign In with Google…", action: signIn)
                            .disabled(draft.clientID.isEmpty || draft.bucket.isEmpty)
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
        .onAppear(perform: revert)
    }

    private var provider: BackupProvider { draft.provider }

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
        let config = draft
        guard let url = backup.googleSignInURL(config) else { return }
        // This throws when the user closes the sign-in window. Then there is nothing to do.
        guard
            let callback = try? await webAuthenticationSession.authenticate(
                using: url, callbackURLScheme: config.googleRedirectScheme)
        else { return }
        await backup.finishGoogleSignIn(callback, config)
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
        if backup.saveKeys(accessKey: accessKey, secretKey: secretKey, for: provider) {
            // Do not keep the secret on screen after it is saved.
            accessKey = ""
            secretKey = ""
        }
    }

    private func save() {
        backup.saveSettings(draft)
        // Read them back, so the screen shows what was saved (spaces around the text are cut off).
        revert()
    }

    /// Drops the changes that are not saved. Also runs when the window opens.
    private func revert() {
        saved = AWSConfig.saved()
        draft = saved
        refreshKeys()
    }

    /// Each provider has its own bucket and folder, so show the saved ones of the provider on screen.
    private func providerChanged() {
        draft = AWSConfig.saved(for: provider)
        refreshKeys()
    }

    /// Each provider has its own keys, so show the ones of the provider on screen.
    private func refreshKeys() {
        backup.refreshKeys(provider)
    }

    private func testConnection() {
        backup.testConnection(draft)
    }

    private func removeKeys() {
        backup.removeKeys(provider)
    }

    private func useDefaultFolder() {
        storageRoot = ""
    }
}
