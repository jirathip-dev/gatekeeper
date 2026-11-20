// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "App",
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.5.0"),
        .package(path: "../Local"),
    ],
    targets: [.executableTarget(name: "App")]
)
