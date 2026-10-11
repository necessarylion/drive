import Foundation

/// One file or folder in the drive.
nonisolated struct DriveItem: Identifiable, Hashable, Sendable {
    let url: URL
    let isFolder: Bool
    let size: Int64
    let modified: Date
    var created: Date = .distantPast
    var isStarred = false

    var id: URL { url }
    var name: String { url.lastPathComponent }
    var kind: FileKind { FileKind(url: url, isFolder: isFolder) }
    var sizeText: String { isFolder ? "—" : size.formatted(.byteCount(style: .file)) }

    /// "Today, 10:42", "Yesterday, 17:30" or "Oct 8, 2026".
    var modifiedText: String {
        let time = modified.formatted(date: .omitted, time: .shortened)
        if Calendar.current.isDateInToday(modified) { return "Today, \(time)" }
        if Calendar.current.isDateInYesterday(modified) { return "Yesterday, \(time)" }
        return modified.formatted(date: .abbreviated, time: .omitted)
    }
}
