import PDFKit
import Quartz
import SwiftUI

/// A window that shows one file. A PDF gets page thumbnails, zoom, search and print.
struct ViewerView: View {
    @Environment(DriveController.self) private var controller
    let url: URL
    @State private var pdf: PDFViewerController
    @State private var isSidebarShown = true
    @State private var searchText = ""

    init(url: URL) {
        self.url = url
        _pdf = State(initialValue: PDFViewerController(url: url))
    }

    var body: some View {
        content
            .navigationTitle(url.lastPathComponent)
            .navigationSubtitle(pdf.isPDF ? "\(pdf.pageText) · Saved to Drive" : "Saved to Drive")
            .toolbar {
                if pdf.isPDF {
                    ToolbarItem(placement: .navigation) {
                        Button("Pages", systemImage: "sidebar.left", action: toggleSidebar)
                    }
                    ToolbarItem {
                        ControlGroup {
                            Button("Zoom Out", systemImage: "minus", action: pdf.zoomOut)
                            Button("Zoom In", systemImage: "plus", action: pdf.zoomIn)
                        }
                    }
                    ToolbarItem {
                        Text(pdf.zoomText)
                            .font(.system(size: 12, weight: .medium))
                            .monospacedDigit()
                    }
                    ToolbarItem {
                        Button("Print", systemImage: "printer", action: pdf.printDocument)
                    }
                }
                ToolbarItem {
                    Button("Download a Copy", systemImage: "arrow.down.to.line", action: downloadCopy)
                }
                ToolbarItem {
                    Button("Show in Finder", systemImage: "info.circle", action: showInFinder)
                }
                ToolbarItem {
                    Menu {
                        ForEach(controller.apps(for: url), id: \.self) { app in
                            Button(controller.appName(app)) {
                                controller.open(url, with: app)
                            }
                        }
                    } label: {
                        Text("Open in \(controller.defaultAppName(for: url))")
                    } primaryAction: {
                        controller.openInDefaultApp(url)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(Theme.accent)
                }
            }
            .tint(Theme.accent)
    }

    @ViewBuilder private var content: some View {
        if pdf.isPDF {
            HStack(spacing: 0) {
                if isSidebarShown {
                    PDFThumbnails(controller: pdf)
                        .frame(width: 150)
                    Divider()
                }
                PDFPage(controller: pdf)
            }
            .searchable(text: $searchText, prompt: "Search in document")
            .onSubmit(of: .search, runSearch)
            .onReceive(NotificationCenter.default.publisher(for: .PDFViewPageChanged, object: pdf.pdfView)) { _ in
                pdf.update()
            }
            .onReceive(NotificationCenter.default.publisher(for: .PDFViewScaleChanged, object: pdf.pdfView)) { _ in
                pdf.update()
            }
        } else {
            QuickLookPage(url: url)
        }
    }

    private func toggleSidebar() {
        isSidebarShown.toggle()
    }

    private func runSearch() {
        pdf.search(searchText)
    }

    private func downloadCopy() {
        controller.askDownloadCopy([url])
    }

    private func showInFinder() {
        controller.showInFinder([url])
    }
}

struct PDFPage: NSViewRepresentable {
    let controller: PDFViewerController

    func makeNSView(context: Context) -> PDFView {
        controller.pdfView
    }

    func updateNSView(_ view: PDFView, context: Context) {}
}

struct PDFThumbnails: NSViewRepresentable {
    let controller: PDFViewerController

    func makeNSView(context: Context) -> PDFThumbnailView {
        controller.thumbnailView
    }

    func updateNSView(_ view: PDFThumbnailView, context: Context) {}
}

/// Shows any file type that Quick Look can show.
struct QuickLookPage: NSViewRepresentable {
    let url: URL

    func makeNSView(context: Context) -> QLPreviewView {
        let view = QLPreviewView(frame: .zero, style: .normal) ?? QLPreviewView()
        view.previewItem = url as NSURL
        return view
    }

    func updateNSView(_ view: QLPreviewView, context: Context) {}
}
