import AppKit
import Observation
import PDFKit

/// Owns the PDF views of one viewer window, so the toolbar can drive them.
@Observable
final class PDFViewerController {
    let pdfView = PDFView()
    let thumbnailView = PDFThumbnailView()
    private(set) var pageText = ""
    private(set) var zoomText = "100%"

    init(url: URL) {
        pdfView.document = PDFDocument(url: url)
        pdfView.autoScales = true
        thumbnailView.pdfView = pdfView
        thumbnailView.thumbnailSize = CGSize(width: 96, height: 124)
        thumbnailView.maximumNumberOfColumns = 1
        thumbnailView.backgroundColor = .clear
        update()
    }

    var isPDF: Bool { pdfView.document != nil }

    /// Reads the page number and zoom again. Call it when the PDF view says they changed.
    func update() {
        guard let document = pdfView.document, let page = pdfView.currentPage else { return }
        pageText = "Page \(document.index(for: page) + 1) of \(document.pageCount)"
        zoomText = "\(Int((pdfView.scaleFactor * 100).rounded()))%"
    }

    func zoomIn() {
        pdfView.zoomIn(nil)
        update()
    }

    func zoomOut() {
        pdfView.zoomOut(nil)
        update()
    }

    /// Jumps to the first place the text is found.
    func search(_ text: String) {
        guard let match = pdfView.document?.findString(text, withOptions: .caseInsensitive).first else { return }
        pdfView.setCurrentSelection(match, animate: true)
        pdfView.go(to: match)
    }

    func printDocument() {
        pdfView.print(with: NSPrintInfo.shared, autoRotate: true)
    }
}
