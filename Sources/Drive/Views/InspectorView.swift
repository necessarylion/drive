import SwiftUI

/// The details pane on the right side of the window.
struct InspectorView: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        if let item = controller.selectedItem {
            // No ScrollView here on purpose. Inside an inspector it swallows mouse clicks on
            // system buttons and menus (seen on macOS 27).
            // ponytail: in a very short window the last rows are cut off.
            ItemDetails(item: item)
                .padding(18)
                .frame(maxHeight: .infinity, alignment: .top)
        } else {
            ContentUnavailableView(
                "Nothing selected", systemImage: "doc.text.magnifyingglass",
                description: Text("Select a file or folder to see its details."))
        }
    }
}

/// Preview, name, actions and facts of one item. Used by the inspector and by column view.
struct ItemDetails: View {
    @Environment(DriveController.self) private var controller
    @Environment(BackupController.self) private var backup
    let item: DriveItem

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            ThumbnailView(item: item, iconSize: 44)
                .frame(height: 170)
                .frame(maxWidth: .infinity)
                .background(item.kind.color.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
                .overlay {
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(Theme.border)
                }

            VStack(alignment: .leading, spacing: 4) {
                Text(item.name)
                    .font(.system(size: 15, weight: .semibold))
                    .lineLimit(2)
                Text(controller.details(for: item))
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            }

            HStack(spacing: 8) {
                Button(controller.section == .trash ? "In Trash" : "Open") {
                    controller.open([item.url])
                }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(controller.section == .trash)
                Menu {
                    ItemMenu(controller: controller, urls: [item.url])
                } label: {
                    Image(systemName: "ellipsis")
                }
                .menuIndicator(.hidden)
                .controlSize(.large)
                .fixedSize()
                .help("More actions")
            }

            if let state = backup.state(of: item) {
                Divider()
                HStack(spacing: 10) {
                    SyncBadge(item: item)
                        .font(.system(size: 16))
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title(of: state))
                            .font(.system(size: 13, weight: .medium))
                        Text(backup.destination)
                            .font(.system(size: 11))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
            }

            Divider()
            VStack(alignment: .leading, spacing: 10) {
                SectionTitle(text: "Information")
                InfoRow(label: "Where", value: controller.whereText(for: item))
                InfoRow(label: "Created", value: item.created.formatted(date: .abbreviated, time: .omitted))
                InfoRow(label: "Modified", value: item.modifiedText)
            }
        }
    }

    private func title(of state: SyncState) -> String {
        switch state {
        case .synced: "Backed up to AWS"
        case .uploading: "Uploading to AWS"
        case .pending: "Not backed up yet"
        }
    }
}

struct InfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .top) {
            Text(label)
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .multilineTextAlignment(.trailing)
        }
        .font(.system(size: 12))
    }
}

/// The wide blue button from the design.
struct PrimaryButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(.white)
            .frame(maxWidth: .infinity)
            .frame(height: 32)
            .background(isEnabled ? Theme.accent : Color.gray, in: RoundedRectangle(cornerRadius: 8))
            .opacity(configuration.isPressed ? 0.75 : 1)
            .contentShape(Rectangle())
    }
}
