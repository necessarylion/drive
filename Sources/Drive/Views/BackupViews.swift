import SwiftUI

/// The backup part of the status bar: a menu, the state, the progress and a pause button.
/// It also owns the backup dialogs.
struct BackupStatusView: View {
    @Environment(BackupController.self) private var backup

    var body: some View {
        @Bindable var backup = backup
        HStack(spacing: 10) {
            Menu {
                Button("Back Up Now", systemImage: "arrow.up.circle", action: backup.backUp)
                    .disabled(backup.isRunning)
                Button("Restore from Backup", systemImage: "arrow.down.circle", action: backup.restore)
                    .disabled(backup.isRunning)
                Divider()
                SettingsLink {
                    Label("Settings…", systemImage: "gearshape")
                }
            } label: {
                Image(systemName: backup.isRunning ? "icloud.and.arrow.up" : "icloud")
                    .foregroundStyle(Theme.accent)
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .help("Backup")

            Text(backup.summaryText)
                .lineLimit(1)
            if backup.isUploading {
                ProgressView(value: backup.fraction)
                    .frame(width: 120)
                Text("\(Int(backup.fraction * 100))%")
                    .fontWeight(.medium)
                    .foregroundStyle(.primary)
                    .monospacedDigit()
            }
            if backup.isConfigured {
                Button(
                    backup.isPaused ? "Resume" : "Pause", systemImage: backup.isPaused ? "play.fill" : "pause.fill",
                    action: backup.togglePause
                )
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
                .help(backup.isPaused ? "Resume the backup" : "Pause the backup")
            }
        }
        .confirmationDialog(
            "\(backup.extrasCount) file(s) in the backup are not on this Mac", isPresented: $backup.isExtrasDialogShown
        ) {
            Button("Keep Them in the Backup", action: backup.backUpAndKeepExtras)
            Button("Remove Them from the Backup", role: .destructive, action: backup.backUpAndRemoveExtras)
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(
                "This happens when you deleted or renamed files here, or when this Mac has not restored yet. "
                    + "Removing from the backup cannot be undone.")
        }
        .alert("Backup problem", isPresented: $backup.isErrorShown) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(backup.errorMessage)
        }
    }
}
