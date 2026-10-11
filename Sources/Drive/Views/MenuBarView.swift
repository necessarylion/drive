import SwiftUI

/// The small window under the Drive icon in the menu bar.
struct MenuBarView: View {
    @Environment(DriveController.self) private var controller
    @Environment(BackupController.self) private var backup
    @Environment(\.openWindow) private var openWindow
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            syncCard
            recentActivity
            Divider()
            footer
        }
        .frame(width: 360)
        .tint(Theme.accent)
    }

    private var header: some View {
        HStack(spacing: 12) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 38, height: 38)
            VStack(alignment: .leading, spacing: 2) {
                Text("Drive")
                    .font(.system(size: 14, weight: .semibold))
                Text(backup.destination)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            Spacer()
            Button("Settings", systemImage: "gearshape", action: showSettings)
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
        }
        .padding(16)
    }

    private var syncCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: backup.isRunning ? "arrow.triangle.2.circlepath" : "checkmark.icloud")
                    .foregroundStyle(Theme.accent)
                Text(backup.summaryText)
                    .font(.system(size: 13, weight: .semibold))
                    .lineLimit(1)
                Spacer()
                if backup.isConfigured {
                    Button(backup.isPaused ? "Resume" : "Pause", action: backup.togglePause)
                        .buttonStyle(.link)
                }
            }
            if backup.isUploading {
                ProgressView(value: backup.fraction)
                Text("\(Int(backup.fraction * 100))%")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.accent.opacity(0.1))
    }

    private var recentActivity: some View {
        VStack(alignment: .leading, spacing: 4) {
            SectionTitle(text: "Recent activity")
                .padding(.horizontal, 16)
                .padding(.top, 12)
            if controller.recentFiles.isEmpty {
                Text("Nothing yet. Add a file to your Drive.")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .padding(16)
            }
            ForEach(controller.recentFiles) { item in
                HStack(spacing: 12) {
                    Image(systemName: item.kind.symbol)
                        .font(.system(size: 17))
                        .foregroundStyle(item.kind.color)
                        .frame(width: 24)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.name)
                            .font(.system(size: 13, weight: .medium))
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Text("Edited \(item.modified.formatted(.relative(presentation: .named)))")
                            .font(.system(size: 11))
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    SyncBadge(item: item)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 7)
            }
        }
        .padding(.bottom, 8)
    }

    private var footer: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("\(controller.usedBytes.formatted(.byteCount(style: .file))) used")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
            HStack(spacing: 8) {
                Button(action: openDrive) {
                    Text("Open Drive")
                        .fontWeight(.semibold)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                Button(action: controller.showDriveInFinder) {
                    Label("Show in Finder", systemImage: "folder")
                        .frame(maxWidth: .infinity)
                }
            }
            .controlSize(.large)
        }
        .padding(16)
    }

    private func openDrive() {
        openWindow(id: AppConstants.mainWindowID)
        NSApp.activate()
    }

    private func showSettings() {
        openSettings()
        NSApp.activate()
    }
}
