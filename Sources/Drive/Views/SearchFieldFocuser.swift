import AppKit

/// Puts the keyboard in the search field of the main window.
/// The field is in the toolbar, and `@FocusState` of SwiftUI does not reach a field there. So this asks AppKit.
final class SearchFieldFocuser {
    func focus() {
        for window in NSApp.windows where window.identifier?.rawValue.hasPrefix(AppConstants.mainWindowID) == true {
            // The search field is the only text field in the toolbar.
            for item in window.toolbar?.items ?? [] {
                guard let view = item.view, let field = textField(in: view) else { continue }
                window.makeKeyAndOrderFront(nil)
                window.makeFirstResponder(field)
                return
            }
        }
    }

    private func textField(in view: NSView) -> NSTextField? {
        if let field = view as? NSTextField, field.isEditable { return field }
        for child in view.subviews {
            if let field = textField(in: child) { return field }
        }
        return nil
    }
}
