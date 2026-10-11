import QuickLook
import SwiftUI

struct ContentView: View {
    @Environment(DriveController.self) private var controller
    @Environment(BackupController.self) private var backup
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        @Bindable var controller = controller
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 200, ideal: 236, max: 300)
        } detail: {
            BrowserView()
                .inspector(isPresented: $controller.isInspectorVisible) {
                    InspectorView()
                        .inspectorColumnWidth(min: 260, ideal: 290, max: 360)
                }
        }
        .searchable(text: $controller.searchText, prompt: "Search in Drive")
        .sheet(isPresented: $controller.isPromptShown) {
            NameSheet()
                .environment(controller)
        }
        .alert("Something went wrong", isPresented: $controller.isErrorShown) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(controller.errorMessage)
        }
        .confirmationDialog(
            "Delete \(controller.pendingDeletion.count) item(s) forever?",
            isPresented: $controller.isDeletionDialogShown
        ) {
            Button("Delete Forever", role: .destructive, action: controller.confirmDeletion)
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("You cannot undo this.")
        }
        .quickLookPreview($controller.previewURL)
        .tint(Theme.accent)
        // Pick up changes made in Finder while the app was in the back.
        .onChange(of: scenePhase, refresh)
        .onChange(of: controller.changeCount, backup.autoBackUp)
        .onChange(of: controller.fileToView, showViewer)
        .task {
            backup.autoBackUp()
        }
    }

    private func refresh() {
        controller.refresh()
        backup.autoBackUp()
    }

    private func showViewer() {
        guard let url = controller.fileToView else { return }
        openWindow(id: AppConstants.viewerWindowID, value: url)
        controller.fileToView = nil
    }
}
