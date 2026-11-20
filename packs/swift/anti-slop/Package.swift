// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "anti-slop-swift",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "anti-slop", targets: ["AntiSlop"])
    ],
    dependencies: [
        .package(url: "https://github.com/swiftlang/swift-syntax.git", exact: "603.0.2")
    ],
    targets: [
        .target(
            name: "AntiSlopCore",
            dependencies: [
                .product(name: "SwiftOperators", package: "swift-syntax"),
                .product(name: "SwiftParser", package: "swift-syntax"),
                .product(name: "SwiftSyntax", package: "swift-syntax"),
            ]
        ),
        .executableTarget(name: "AntiSlop", dependencies: ["AntiSlopCore"]),
    ]
)
