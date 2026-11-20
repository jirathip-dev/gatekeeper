import Foundation

let decoded = try! JSONDecoder().decode([Int].self, from: Data("[1]".utf8))
