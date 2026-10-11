import SwiftUI

/// The dialog for a new folder (name and color) and for a rename.
struct NameSheet: View {
    @Environment(DriveController.self) private var controller

    var body: some View {
        @Bindable var controller = controller
        VStack(alignment: .leading, spacing: 20) {
            HStack(spacing: 14) {
                Image(systemName: controller.prompt.symbol)
                    .font(.system(size: 18, weight: .medium))
                    .foregroundStyle(Theme.accent)
                    .frame(width: 44, height: 44)
                    .background(Theme.accent.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
                VStack(alignment: .leading, spacing: 3) {
                    Text(controller.prompt.title)
                        .font(.system(size: 15, weight: .semibold))
                    Text(subtitle)
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                }
            }

            VStack(alignment: .leading, spacing: 6) {
                Text("Name")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                TextField("Name", text: $controller.promptText)
                    .textFieldStyle(.roundedBorder)
                    .controlSize(.large)
                    .onSubmit(controller.confirmPrompt)
            }

            if controller.prompt.isNewFolder {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Color")
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                    HStack(spacing: 10) {
                        ForEach(FolderColor.allCases) { color in
                            Circle()
                                .fill(color.color)
                                .frame(width: 16, height: 16)
                                .padding(4)
                                .overlay {
                                    Circle()
                                        .stroke(color.color, lineWidth: controller.promptColor == color ? 2 : 0)
                                }
                                .contentShape(Circle())
                                .onTapGesture {
                                    controller.promptColor = color
                                }
                                .help(color.title)
                        }
                    }
                }
            }

            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: controller.cancelPrompt)
                    .keyboardShortcut(.cancelAction)
                Button(controller.prompt.confirmLabel, action: controller.confirmPrompt)
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .tint(Theme.accent)
            }
            .controlSize(.large)
        }
        .padding(24)
        .frame(width: 400)
    }

    private var subtitle: String {
        controller.prompt.isNewFolder ? "It will be created in \(controller.title)." : "Type the new name."
    }
}
