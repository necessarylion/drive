import SwiftUI

/// The middle of the window: the toolbar, the items and the status bar.
struct BrowserView: View {
    @Environment(DriveController.self) private var controller

    private let folderColumns = [GridItem(.adaptive(minimum: 180), spacing: 12)]
    private let tileColumns = [GridItem(.adaptive(minimum: AppConstants.tileMinWidth), spacing: 12)]

    var body: some View {
        @Bindable var controller = controller
        content
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(.background)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                StatusBar()
            }
            .dropDestination(for: URL.self) { urls, _ in
                controller.drop(urls, on: controller.currentFolder)
            }
            .contextMenu {
                NewItemMenu(controller: controller)
            }
            .navigationTitle(controller.title)
            .toolbar(removing: .title)
            .toolbar {
                ToolbarItem(placement: .navigation) {
                    HStack(spacing: 4) {
                        Button("Back", systemImage: "chevron.left", action: controller.goBack)
                            .disabled(!controller.canGoBack)
                        Button("Forward", systemImage: "chevron.right", action: controller.goForward)
                            .disabled(!controller.canGoForward)
                        BreadcrumbBar()
                    }
                }
                ToolbarItem {
                    Picker("View", selection: $controller.viewMode) {
                        Label("List", systemImage: "list.bullet").tag(ViewMode.list)
                        Label("Grid", systemImage: "square.grid.2x2").tag(ViewMode.grid)
                        Label("Columns", systemImage: "rectangle.split.3x1").tag(ViewMode.columns)
                    }
                    .pickerStyle(.segmented)
                }
                ToolbarItem {
                    Menu("Sort", systemImage: "arrow.up.arrow.down") {
                        Picker("Sort by", selection: $controller.sortField) {
                            ForEach(SortField.allCases) { field in
                                Text(field.title).tag(field)
                            }
                        }
                        .pickerStyle(.inline)
                        Toggle("Ascending", isOn: $controller.isSortAscending)
                    }
                }
                ToolbarItem {
                    Button("Inspector", systemImage: "sidebar.right", action: controller.toggleInspector)
                        .help("Show or hide the details")
                }
                ToolbarItem {
                    if controller.section == .trash {
                        Button("Empty Trash", systemImage: "trash.slash", action: controller.askEmptyTrash)
                            .disabled(controller.items.isEmpty)
                    } else {
                        Menu {
                            NewItemMenu(controller: controller)
                        } label: {
                            Label("New", systemImage: "plus")
                                .labelStyle(.titleAndIcon)
                        }
                        .menuStyle(.button)
                        .menuIndicator(.hidden)
                        .buttonStyle(.borderedProminent)
                        .tint(Theme.accent)
                        .disabled(!controller.canAddFiles)
                    }
                }
            }
    }

    @ViewBuilder private var content: some View {
        if controller.isColumnMode {
            ColumnBrowserView()
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    header
                    if controller.items.isEmpty {
                        EmptyStateView()
                            .frame(maxWidth: .infinity, minHeight: 320)
                    } else {
                        folders
                        files
                    }
                }
                .padding(.horizontal, 28)
                .padding(.vertical, 22)
            }
            .contentShape(Rectangle())
            .onTapGesture(perform: controller.clearSelection)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(controller.title)
                .font(.system(size: 22, weight: .semibold))
            Text(controller.headerSubtitle)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder private var folders: some View {
        let folders = controller.items.filter(\.isFolder)
        if !folders.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                SectionTitle(text: "Folders")
                LazyVGrid(columns: folderColumns, alignment: .leading, spacing: 12) {
                    ForEach(folders) { folder in
                        FolderCard(item: folder)
                    }
                }
            }
        }
    }

    @ViewBuilder private var files: some View {
        let files = controller.items.filter({ !$0.isFolder })
        if !files.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                SectionTitle(text: "Files")
                if controller.viewMode == .grid {
                    LazyVGrid(columns: tileColumns, alignment: .leading, spacing: 12) {
                        ForEach(files) { file in
                            FileTile(item: file)
                        }
                    }
                } else {
                    VStack(spacing: 0) {
                        FileListHeader()
                        ForEach(files) { file in
                            FileRow(item: file)
                        }
                    }
                }
            }
        }
    }
}

struct SectionTitle: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(.secondary)
    }
}

struct BreadcrumbBar: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        HStack(spacing: 6) {
            if controller.section == .myDrive {
                crumb(DriveSection.myDrive.title, level: 0)
                ForEach(Array(controller.path.enumerated()), id: \.element) { index, folder in
                    Image(systemName: "chevron.right")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(.tertiary)
                    crumb(folder.lastPathComponent, level: index + 1)
                }
            } else {
                Text(controller.section.title)
                    .fontWeight(.semibold)
            }
        }
        .font(.system(size: 13))
        .padding(.horizontal, 8)
    }

    private func crumb(_ title: String, level: Int) -> some View {
        let isCurrent = level == controller.path.count
        return Button(title) {
            controller.goToPathLevel(level)
        }
        .buttonStyle(.plain)
        .fontWeight(isCurrent ? .semibold : .regular)
        .foregroundStyle(isCurrent ? .primary : .secondary)
        .lineLimit(1)
    }
}

struct EmptyStateView: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        if controller.searchText.isEmpty {
            ContentUnavailableView {
                Label(controller.section.emptyTitle, systemImage: controller.section.symbol)
            } description: {
                Text(controller.section.emptyMessage)
            } actions: {
                if controller.canAddFiles {
                    Button("Upload Files…", action: controller.uploadFiles)
                }
            }
        } else {
            ContentUnavailableView.search(text: controller.searchText)
        }
    }
}

/// The bar at the bottom: the item count on the left, the backup on the right.
struct StatusBar: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        HStack(spacing: 10) {
            Text(controller.statusText)
            if controller.isBusy {
                ProgressView()
                    .controlSize(.small)
            }
            Spacer()
            BackupStatusView()
        }
        .font(.system(size: 12))
        .foregroundStyle(.secondary)
        .padding(.horizontal, 28)
        .frame(height: 40)
        .background(.bar)
        .overlay(alignment: .top) {
            Divider()
        }
    }
}
