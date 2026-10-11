import SwiftUI

/// Colors from the Pencil design. Text and backgrounds use system colors, so dark mode works.
enum Theme {
    static let accent = Color(red: 0.24, green: 0.35, blue: 1.0)
    static let success = Color(red: 0.12, green: 0.62, blue: 0.42)
    static let purple = Color(red: 0.69, green: 0.36, blue: 0.86)
    static let orange = Color(red: 0.95, green: 0.55, blue: 0.15)
    static let pink = Color(red: 0.91, green: 0.25, blue: 0.45)
    static let red = Color(red: 0.90, green: 0.27, blue: 0.27)
    static let border = Color(nsColor: .separatorColor)
}

extension FolderColor {
    var color: Color {
        switch self {
        case .blue: Theme.accent
        case .purple: Theme.purple
        case .green: Theme.success
        case .orange: Theme.orange
        case .pink: Theme.pink
        case .gray: .gray
        }
    }
}

extension FileKind {
    var color: Color {
        switch self {
        case .folder, .image, .text: Theme.accent
        case .pdf: Theme.red
        case .video, .audio: Theme.pink
        case .sheet: Theme.success
        case .slides: Theme.orange
        case .design: Theme.purple
        case .archive, .other: .gray
        }
    }
}

extension StorageGroup {
    var color: Color {
        switch self {
        case .documents: Theme.accent
        case .media: Theme.orange
        case .other: Theme.purple
        }
    }
}
