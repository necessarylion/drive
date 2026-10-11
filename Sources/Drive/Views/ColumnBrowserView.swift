import SwiftUI

/// Column view: one column per open folder, and a preview of the picked file at the end.
struct ColumnBrowserView: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: 0) {
                ForEach(Array(controller.columns.enumerated()), id: \.offset) { depth, items in
                    ScrollView {
                        LazyVStack(spacing: 1) {
                            ForEach(items) { item in
                                ColumnRow(item: item, depth: depth)
                            }
                        }
                        .padding(8)
                    }
                    .frame(width: AppConstants.columnWidth)
                    Divider()
                }
                if let item = controller.selectedItem {
                    ScrollView {
                        ItemDetails(item: item)
                            .padding(28)
                    }
                    .frame(width: 400)
                }
            }
        }
    }
}

struct ColumnRow: View {
    @Environment(DriveController.self) private var controller
    let item: DriveItem
    let depth: Int

    var body: some View {
        let isSelected = controller.selection.contains(item.url)
        let isOpen = controller.path.count > depth && controller.path[depth] == item.url
        HStack(spacing: 8) {
            Image(systemName: item.kind.symbol)
                .font(.system(size: 13))
                .foregroundStyle(isSelected ? Color.white : iconColor)
                .frame(width: 18)
            Text(item.name)
                .font(.system(size: 13))
                .foregroundStyle(isSelected ? Color.white : Color.primary)
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer(minLength: 0)
            if item.isFolder {
                Image(systemName: "chevron.right")
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.horizontal, 8)
        .frame(height: 28)
        .background(background(isSelected: isSelected, isOpen: isOpen), in: RoundedRectangle(cornerRadius: 6))
        .contentShape(Rectangle())
        .gesture(
            TapGesture(count: 2).onEnded {
                controller.open([item.url])
            }
        )
        .simultaneousGesture(
            TapGesture().onEnded {
                controller.selectInColumn(item, depth: depth)
            }
        )
        .contextMenu {
            ItemMenu(controller: controller, urls: [item.url])
        }
    }

    private var iconColor: Color {
        item.isFolder ? controller.color(of: item.url).color : item.kind.color
    }

    private func background(isSelected: Bool, isOpen: Bool) -> Color {
        if isSelected { return Theme.accent }
        return isOpen ? Color.primary.opacity(0.08) : Color.clear
    }
}
