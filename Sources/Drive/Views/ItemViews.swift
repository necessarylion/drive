import QuickLookThumbnailing
import SwiftUI

/// A folder shown as a card: a colored icon, the name, and what is inside.
struct FolderCard: View {
    @Environment(DriveController.self) private var controller
    let item: DriveItem

    var body: some View {
        let isSelected = controller.selection.contains(item.url)
        let color = controller.color(of: item.url).color
        HStack(spacing: 12) {
            Image(systemName: "folder")
                .font(.system(size: 16, weight: .medium))
                .foregroundStyle(color)
                .frame(width: 34, height: 34)
                .background(color.opacity(0.14), in: RoundedRectangle(cornerRadius: 8))
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 4) {
                    Text(item.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                    StarMark(item: item)
                }
                Text(summaryText)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            Spacer(minLength: 0)
        }
        .padding(11)
        .background(isSelected ? Theme.accent.opacity(0.12) : Color.clear, in: RoundedRectangle(cornerRadius: 10))
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(isSelected ? Theme.accent : Theme.border)
        }
        .modifier(ItemInteractions(item: item))
    }

    private var summaryText: String {
        controller.summaries[item.url]?.text ?? "Folder"
    }
}

/// The titles above the file list. A click sorts by that column.
struct FileListHeader: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        HStack(spacing: 0) {
            title(.name)
                .frame(maxWidth: .infinity, alignment: .leading)
            title(.modified)
                .frame(width: AppConstants.dateColumnWidth, alignment: .leading)
            title(.size)
                .frame(width: AppConstants.sizeColumnWidth, alignment: .leading)
            Spacer()
                .frame(width: AppConstants.badgeColumnWidth)
        }
        .padding(.horizontal, 12)
        .frame(height: 30)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }

    private func title(_ field: SortField) -> some View {
        let isActive = controller.sortField == field
        return Button {
            controller.sort(by: field)
        } label: {
            HStack(spacing: 4) {
                Text(field.title)
                if isActive {
                    Image(systemName: controller.isSortAscending ? "chevron.up" : "chevron.down")
                        .font(.system(size: 8, weight: .bold))
                }
            }
            .font(.system(size: 11, weight: isActive ? .semibold : .regular))
            .foregroundStyle(isActive ? .primary : .secondary)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

/// One file in list view.
struct FileRow: View {
    @Environment(DriveController.self) private var controller
    let item: DriveItem

    var body: some View {
        let isSelected = controller.selection.contains(item.url)
        HStack(spacing: 0) {
            HStack(spacing: 12) {
                FileIcon(kind: item.kind)
                Text(item.name)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)
                    .truncationMode(.middle)
                StarMark(item: item)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(item.modifiedText)
                .frame(width: AppConstants.dateColumnWidth, alignment: .leading)
            Text(item.sizeText)
                .frame(width: AppConstants.sizeColumnWidth, alignment: .leading)
            SyncBadge(item: item)
                .frame(width: AppConstants.badgeColumnWidth)
        }
        .font(.system(size: 12))
        .foregroundStyle(.secondary)
        .padding(.horizontal, 12)
        .frame(height: 42)
        .background(isSelected ? Theme.accent.opacity(0.12) : Color.clear, in: RoundedRectangle(cornerRadius: 8))
        .modifier(ItemInteractions(item: item))
    }
}

/// One file in grid view.
struct FileTile: View {
    @Environment(DriveController.self) private var controller
    let item: DriveItem

    var body: some View {
        let isSelected = controller.selection.contains(item.url)
        VStack(alignment: .leading, spacing: 8) {
            ThumbnailView(item: item)
                .frame(height: 112)
                .background(item.kind.color.opacity(0.1), in: RoundedRectangle(cornerRadius: 8))
            HStack(alignment: .top, spacing: 6) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(item.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Text(item.sizeText)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                SyncBadge(item: item)
            }
            .padding(.horizontal, 2)
        }
        .padding(8)
        .background(isSelected ? Theme.accent.opacity(0.12) : Color.clear, in: RoundedRectangle(cornerRadius: 10))
        .modifier(ItemInteractions(item: item))
    }
}

struct FileIcon: View {
    let kind: FileKind

    var body: some View {
        Image(systemName: kind.symbol)
            .font(.system(size: 15))
            .foregroundStyle(kind.color)
            .frame(width: 20)
    }
}

struct StarMark: View {
    let item: DriveItem

    var body: some View {
        if item.isStarred {
            Image(systemName: "star.fill")
                .font(.system(size: 10))
                .foregroundStyle(.yellow)
        }
    }
}

/// The small icon that tells if a file is in the backup.
struct SyncBadge: View {
    @Environment(BackupController.self) private var backup
    let item: DriveItem

    var body: some View {
        switch backup.state(of: item) {
        case .synced:
            Image(systemName: "checkmark.circle")
                .foregroundStyle(Theme.success)
                .help("Backed up")
        case .uploading:
            Image(systemName: "arrow.triangle.2.circlepath")
                .foregroundStyle(Theme.accent)
                .help("Uploading")
        case .pending:
            Image(systemName: "icloud.and.arrow.up")
                .foregroundStyle(.tertiary)
                .help("Not backed up yet")
        case nil:
            EmptyView()
        }
    }
}

/// Shows a real preview of the file when there is one. Shows the type icon if not.
struct ThumbnailView: View {
    let item: DriveItem
    var iconSize: CGFloat = 28
    @State private var thumbnail: NSImage?

    var body: some View {
        Color.clear
            .overlay {
                if let thumbnail {
                    Image(nsImage: thumbnail)
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                } else {
                    Image(systemName: item.kind.symbol)
                        .font(.system(size: iconSize))
                        .foregroundStyle(item.kind.color)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .task(id: item) {
                await loadThumbnail()
            }
    }

    private func loadThumbnail() async {
        if item.isFolder { return }
        let request = QLThumbnailGenerator.Request(
            fileAt: item.url, size: AppConstants.thumbnailSize, scale: 2, representationTypes: .thumbnail)
        thumbnail = try? await QLThumbnailGenerator.shared.generateBestRepresentation(for: request).nsImage
    }
}

/// Clicks, the right-click menu, and drag and drop. Shared by cards, rows and tiles.
struct ItemInteractions: ViewModifier {
    @Environment(DriveController.self) private var controller
    let item: DriveItem

    func body(content: Content) -> some View {
        let base =
            content
            .contentShape(Rectangle())
            .gesture(
                TapGesture(count: 2).onEnded {
                    controller.open([item.url])
                }
            )
            .simultaneousGesture(
                TapGesture().onEnded {
                    controller.select(item.url)
                }
            )
            .contextMenu {
                ItemMenu(controller: controller, urls: controller.targets(for: item.url))
            }
            .help(item.name)

        if controller.section == .trash {
            base
        } else if item.isFolder {
            base
                .draggable(item.url)
                .dropDestination(for: URL.self) { urls, _ in
                    controller.drop(urls, on: item.url)
                }
        } else {
            base.draggable(item.url)
        }
    }
}

/// The right-click menu of one or more items.
struct ItemMenu: View {
    let controller: DriveController
    let urls: Set<URL>

    var body: some View {
        if controller.section == .trash {
            Button("Put Back", systemImage: "arrow.uturn.backward") {
                controller.restore(urls)
            }
            Button("Delete Forever…", systemImage: "trash.slash", role: .destructive) {
                controller.askDeleteForever(urls)
            }
        } else {
            let areFolders = controller.areFolders(urls)
            let starred = controller.areStarred(urls)
            Button("Open", systemImage: "arrow.up.forward.square") {
                controller.open(urls)
            }
            if let url = urls.first, urls.count == 1, !areFolders {
                Menu("Open With", systemImage: "macwindow") {
                    ForEach(controller.apps(for: url), id: \.self) { app in
                        Button(controller.appName(app)) {
                            controller.open(url, with: app)
                        }
                    }
                }
            }
            Button("Quick Look", systemImage: "eye") {
                controller.preview(urls)
            }
            Divider()
            Button("Rename…", systemImage: "pencil") {
                controller.askRename(urls)
            }
            .disabled(urls.count != 1)
            Button("Duplicate", systemImage: "plus.square.on.square") {
                controller.duplicate(urls)
            }
            Button("Move to…", systemImage: "folder") {
                controller.askMove(urls)
            }
            Button(starred ? "Remove Star" : "Add Star", systemImage: starred ? "star.slash" : "star") {
                controller.toggleStar(urls)
            }
            if areFolders {
                let pinned = controller.arePinned(urls)
                Button(pinned ? "Unpin from Sidebar" : "Pin to Sidebar", systemImage: pinned ? "pin.slash" : "pin") {
                    controller.togglePin(urls)
                }
                Menu("Folder Color", systemImage: "paintpalette") {
                    ForEach(FolderColor.allCases) { color in
                        Button(color.title) {
                            controller.setColor(color, for: urls)
                        }
                    }
                }
            }
            Divider()
            Button("Download a Copy…", systemImage: "arrow.down.to.line") {
                controller.askDownloadCopy(urls)
            }
            Button("Show in Finder", systemImage: "finder") {
                controller.showInFinder(urls)
            }
            Button("Get Info", systemImage: "info.circle") {
                controller.showInfo(urls)
            }
            Divider()
            Button("Move to Trash", systemImage: "trash", role: .destructive) {
                controller.trash(urls)
            }
        }
    }
}

/// The menu for making new things. Used by the New button and by a right click on empty space.
struct NewItemMenu: View {
    let controller: DriveController

    var body: some View {
        if controller.canAddFiles {
            Button("New Folder", systemImage: "folder.badge.plus", action: controller.askNewFolder)
            Divider()
            Button("Upload Files…", systemImage: "square.and.arrow.up", action: controller.uploadFiles)
            Button("Upload Folder…", systemImage: "square.and.arrow.up.on.square", action: controller.uploadFolder)
            Divider()
            Button("Text Document", systemImage: "doc.text", action: controller.newTextDocument)
            Button("Spreadsheet", systemImage: "tablecells", action: controller.newSpreadsheet)
        }
    }
}
