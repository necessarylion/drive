import Foundation

/// Extra facts the file system does not keep. Saved as one JSON file.
nonisolated struct DriveMetadata: Codable, Sendable {
    /// Paths of starred items.
    var starred: Set<String> = []
    /// Trash slot id -> the path the item had before it was trashed.
    var trashOrigins: [String: String] = [:]
    /// Paths of folders pinned to the sidebar, in the order they were pinned.
    var pinned: [String] = []
    /// Folder path -> its color.
    var folderColors: [String: FolderColor] = [:]

    init() {}

    /// Every field is optional in the file, so a file saved by an older version still loads.
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        starred = try values.decodeIfPresent(Set<String>.self, forKey: .starred) ?? []
        trashOrigins = try values.decodeIfPresent([String: String].self, forKey: .trashOrigins) ?? [:]
        pinned = try values.decodeIfPresent([String].self, forKey: .pinned) ?? []
        folderColors = try values.decodeIfPresent([String: FolderColor].self, forKey: .folderColors) ?? [:]
    }

    /// Keeps stars, pins and colors on an item, and on everything inside it, after a move or rename.
    mutating func repath(from oldPath: String, to newPath: String) {
        var newStarred: Set<String> = []
        for path in starred {
            newStarred.insert(moved(path, from: oldPath, to: newPath))
        }
        starred = newStarred

        for index in pinned.indices {
            pinned[index] = moved(pinned[index], from: oldPath, to: newPath)
        }

        var newColors: [String: FolderColor] = [:]
        for (path, color) in folderColors {
            newColors[moved(path, from: oldPath, to: newPath)] = color
        }
        folderColors = newColors
    }

    private func moved(_ path: String, from oldPath: String, to newPath: String) -> String {
        if path == oldPath || path.hasPrefix(oldPath + "/") {
            return newPath + path.dropFirst(oldPath.count)
        }
        return path
    }
}
