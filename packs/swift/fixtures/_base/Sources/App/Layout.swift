public struct Layout: Sendable {
    public let spacing: Double
    public let height: Double

    public init(spacing: Double, height: Double) {
        self.spacing = spacing
        self.height = height
    }

    public static let row = Layout(spacing: 0, height: 44)
}
