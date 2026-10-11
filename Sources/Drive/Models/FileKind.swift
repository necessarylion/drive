import Foundation
import UniformTypeIdentifiers

/// The rough type of a file. It picks the icon and its color.
nonisolated enum FileKind: Sendable {
    case folder, pdf, image, video, audio, sheet, slides, text, archive, design, other

    init(url: URL, isFolder: Bool) {
        if isFolder {
            self = .folder
            return
        }
        let ext = url.pathExtension.lowercased()
        let type = UTType(filenameExtension: ext) ?? .data
        if ["fig", "sketch", "psd", "ai", "xd"].contains(ext) {
            self = .design
        } else if ["xlsx", "xls", "numbers", "csv"].contains(ext) || type.conforms(to: .spreadsheet) {
            self = .sheet
        } else if ["key", "pptx", "ppt"].contains(ext) || type.conforms(to: .presentation) {
            self = .slides
        } else if type.conforms(to: .pdf) {
            self = .pdf
        } else if type.conforms(to: .image) {
            self = .image
        } else if type.conforms(to: .movie) {
            self = .video
        } else if type.conforms(to: .audio) {
            self = .audio
        } else if type.conforms(to: .archive) {
            self = .archive
        } else if ["doc", "docx", "pages", "rtf"].contains(ext) || type.conforms(to: .text) {
            self = .text
        } else {
            self = .other
        }
    }

    var symbol: String {
        switch self {
        case .folder: "folder"
        case .pdf: "doc.richtext"
        case .image: "photo"
        case .video: "film"
        case .audio: "waveform"
        case .sheet: "tablecells"
        case .slides: "rectangle.on.rectangle.angled"
        case .text: "doc.text"
        case .archive: "doc.zipper"
        case .design: "pencil.and.outline"
        case .other: "doc"
        }
    }

    var storageGroup: StorageGroup {
        switch self {
        case .pdf, .sheet, .slides, .text: .documents
        case .image, .video, .audio, .design: .media
        case .folder, .archive, .other: .other
        }
    }
}
