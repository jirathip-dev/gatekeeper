// swift-tools-version: 6.0
import Foundation
import PackageDescription

let strict: [SwiftSetting] = []

let package = Package(
    name: "App",
    targets: [
        .target(name: "App", swiftSettings: strict),
        .testTarget(name: "AppTests", dependencies: ["App"], swiftSettings: strict),
    ]
)
