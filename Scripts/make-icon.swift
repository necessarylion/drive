import AppKit

// Turns the logo from the Pencil design into the macOS icon set.
// macOS icons keep an empty margin around the plate, so the logo is drawn a bit smaller than the canvas.
// Usage: swift Scripts/make-icon.swift <logo.png> <output.iconset>
final class IconRenderer {
    private let logo: NSImage
    /// The plate is 824 of 1024 points wide in Apple's icon grid.
    private let plateShare: CGFloat = 824.0 / 1024.0

    init(logo: NSImage) {
        self.logo = logo
    }

    func png(pixels: Int) -> Data {
        let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels, bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        let side = CGFloat(pixels) * plateShare
        let margin = (CGFloat(pixels) - side) / 2
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
        NSGraphicsContext.current?.imageInterpolation = .high
        logo.draw(in: NSRect(x: margin, y: margin, width: side, height: side))
        NSGraphicsContext.restoreGraphicsState()
        return bitmap.representation(using: .png, properties: [:])!
    }
}

guard CommandLine.arguments.count == 3, let logo = NSImage(contentsOfFile: CommandLine.arguments[1]) else {
    print("Usage: swift Scripts/make-icon.swift <logo.png> <output.iconset>")
    exit(1)
}

let iconset = URL(filePath: CommandLine.arguments[2])
let renderer = IconRenderer(logo: logo)

try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
for points in [16, 32, 128, 256, 512] {
    try renderer.png(pixels: points).write(to: iconset.appending(path: "icon_\(points)x\(points).png"))
    try renderer.png(pixels: points * 2).write(to: iconset.appending(path: "icon_\(points)x\(points)@2x.png"))
}
