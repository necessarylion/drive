import SwiftUI

struct SidebarView: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        @Bindable var controller = controller
        List(selection: $controller.sidebarSelection) {
            Section("Drive") {
                ForEach(DriveSection.allCases) { section in
                    Label {
                        Text(section.title)
                    } icon: {
                        if section == .myDrive {
                            // My Drive shows the app logo.
                            Image(nsImage: NSApp.applicationIconImage)
                                .resizable()
                                .frame(width: 20, height: 20)
                        } else {
                            Image(systemName: section.symbol)
                        }
                    }
                    .modifier(SidebarRowTap(item: .section(section)))
                    .tag(SidebarItem.section(section))
                }
            }
            if !controller.pinned.isEmpty {
                Section("Pinned folders") {
                    ForEach(controller.pinned) { folder in
                        Label {
                            Text(folder.url.lastPathComponent)
                        } icon: {
                            Circle()
                                .fill(folder.color.color)
                                .frame(width: 9, height: 9)
                        }
                        .modifier(SidebarRowTap(item: .folder(folder.url)))
                        .tag(SidebarItem.folder(folder.url))
                        .contextMenu {
                            Button("Unpin", systemImage: "pin.slash") {
                                controller.togglePin([folder.url])
                            }
                        }
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            StorageFooter()
        }
    }
}

/// A list only tells us when the picked row changes. This also catches a click on the row that is
/// already picked, so "My Drive" always goes back to the top folder.
struct SidebarRowTap: ViewModifier {
    @Environment(DriveController.self) private var controller
    let item: SidebarItem

    func body(content: Content) -> some View {
        content
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .simultaneousGesture(
                TapGesture().onEnded {
                    controller.sidebarSelection = item
                }
            )
    }
}

/// The storage bar and the backup line at the bottom of the sidebar.
struct StorageFooter: View {
    @Environment(DriveController.self) private var controller
    @Environment(BackupController.self) private var backup

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Divider()
            Text("Storage")
                .font(.system(size: 12, weight: .semibold))
            GeometryReader { geometry in
                HStack(spacing: 2) {
                    ForEach(controller.slices) { slice in
                        Rectangle()
                            .fill(slice.group.color)
                            .frame(width: width(of: slice, in: geometry.size.width))
                    }
                    Rectangle()
                        .fill(.quaternary)
                }
            }
            .frame(height: 4)
            .clipShape(Capsule())
            Text("\(controller.usedBytes.formatted(.byteCount(style: .file))) used")
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
            HStack(spacing: 6) {
                Image(systemName: backup.isRunning ? "arrow.triangle.2.circlepath.icloud" : "checkmark.icloud")
                    .foregroundStyle(backup.isConfigured ? Theme.success : Color.secondary)
                Text(backup.summaryText)
                    .lineLimit(1)
            }
            .font(.system(size: 11))
            .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 14)
        .padding(.bottom, 12)
    }

    /// Each slice gets its share of the bar. The bar is full when the drive has files.
    private func width(of slice: StorageSlice, in total: CGFloat) -> CGFloat {
        var sum: Int64 = 0
        for other in controller.slices {
            sum += other.size
        }
        if sum == 0 { return 0 }
        return max(3, total * CGFloat(slice.size) / CGFloat(sum) - 2)
    }
}
