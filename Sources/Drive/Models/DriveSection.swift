import Foundation

/// The lists in the sidebar.
nonisolated enum DriveSection: String, CaseIterable, Identifiable, Sendable {
    case myDrive, recent, starred, trash

    var id: String { rawValue }

    var title: String {
        switch self {
        case .myDrive: "My Drive"
        case .recent: "Recents"
        case .starred: "Starred"
        case .trash: "Trash"
        }
    }

    var symbol: String {
        switch self {
        case .myDrive: "externaldrive"
        case .recent: "clock.arrow.circlepath"
        case .starred: "star"
        case .trash: "trash"
        }
    }

    var emptyTitle: String {
        switch self {
        case .myDrive: "This folder is empty"
        case .recent: "No recent files"
        case .starred: "No starred items"
        case .trash: "Trash is empty"
        }
    }

    var emptyMessage: String {
        switch self {
        case .myDrive: "Drop files here, or upload them."
        case .recent: "Files you add or change show up here."
        case .starred: "Add a star to an item to find it fast."
        case .trash: "Items you move to the trash show up here."
        }
    }
}

nonisolated enum ViewMode: String, Sendable {
    case list, grid, columns
}

/// What the name dialog is asking for.
nonisolated enum NamePrompt: Sendable {
    case newFolder
    case rename(URL)

    var title: String {
        switch self {
        case .newFolder: "New folder"
        case .rename: "Rename"
        }
    }

    var confirmLabel: String {
        switch self {
        case .newFolder: "Create"
        case .rename: "Rename"
        }
    }

    var symbol: String {
        switch self {
        case .newFolder: "folder.badge.plus"
        case .rename: "pencil"
        }
    }

    var isNewFolder: Bool {
        if case .newFolder = self { return true }
        return false
    }
}
