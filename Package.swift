// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "Drive",
    platforms: [.macOS(.v15)],
    targets: [
        .executableTarget(
            name: "Drive",
            swiftSettings: [.defaultIsolation(MainActor.self)]
        ),
        .testTarget(name: "DriveTests", dependencies: ["Drive"]),
    ]
)
