import Foundation

/// A place in the app: a sidebar list, and the open folders under it.
nonisolated struct Location: Equatable, Sendable {
    var section: DriveSection
    var path: [URL] = []
    /// The pinned folder the user came in through, if any. It stays picked in the sidebar.
    var pinned: URL?
}

/// A row the user can pick in the sidebar.
nonisolated enum SidebarItem: Hashable, Sendable {
    case section(DriveSection)
    case folder(URL)
}

nonisolated struct FolderSummary: Sendable {
    let count: Int
    let size: Int64

    /// "48 items · 1.1 GB"
    var text: String { "\(Plural.items(count)) · \(size.formatted(.byteCount(style: .file)))" }
}

nonisolated enum Plural {
    static func items(_ count: Int) -> String {
        count == 1 ? "1 item" : "\(count) items"
    }
}

nonisolated enum FolderColor: String, CaseIterable, Codable, Identifiable, Sendable {
    case blue, purple, green, orange, pink, gray

    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

nonisolated struct PinnedFolder: Identifiable, Sendable {
    let url: URL
    let color: FolderColor

    var id: URL { url }
}

/// The parts of the storage bar.
nonisolated enum StorageGroup: CaseIterable, Sendable {
    case documents, media, other
}

nonisolated struct StorageSlice: Identifiable, Sendable {
    let group: StorageGroup
    let size: Int64

    var id: StorageGroup { group }
}

/// Where a file stands with the backup.
nonisolated enum SyncState: Sendable {
    case synced, pending, uploading
}

nonisolated enum SortField: String, CaseIterable, Identifiable, Sendable {
    case name, modified, size

    var id: String { rawValue }

    var title: String {
        switch self {
        case .name: "Name"
        case .modified: "Last modified"
        case .size: "Size"
        }
    }

    func comparator(ascending: Bool) -> KeyPathComparator<DriveItem> {
        let order: SortOrder = ascending ? .forward : .reverse
        switch self {
        case .name: return KeyPathComparator(\.name, comparator: .localizedStandard, order: order)
        case .modified: return KeyPathComparator(\.modified, order: order)
        case .size: return KeyPathComparator(\.size, order: order)
        }
    }
}
