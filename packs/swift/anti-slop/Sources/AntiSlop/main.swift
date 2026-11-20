import AntiSlopCore
import Foundation

let usage = """
    anti-slop [--enable=RULE,RULE] PATH ...
    anti-slop --list-rules

    The shared rules always run; --enable switches extra rules on.
    Only .swift files are linted; .build, .git, .swiftpm and DerivedData are skipped.
    """

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("[anti-slop] \(message)\n".utf8))
    exit(2)
}

func swiftFiles(under path: String) -> [String] {
    var isDirectory: ObjCBool = false
    guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) else {
        fail("no such path: \(path)")
    }
    guard isDirectory.boolValue else {
        return path.hasSuffix(".swift") ? [path] : []
    }
    let skipped: Set<String> = [".git", ".build", ".swiftpm", "DerivedData"]
    var results: [String] = []
    let enumerator = FileManager.default.enumerator(atPath: path)
    while let element = enumerator?.nextObject() as? String {
        if let last = element.split(separator: "/").last, skipped.contains(String(last)) {
            enumerator?.skipDescendants()
            continue
        }
        if element.hasSuffix(".swift") {
            results.append((path as NSString).appendingPathComponent(element))
        }
    }
    return results.sorted()
}

let arguments = CommandLine.arguments.dropFirst()
if arguments.contains("--list-rules") {
    for rule in SlopRule.allRules { print("\(rule.id)\n    \(rule.summary)") }
    for rule in SlopRule.extraRules { print("\(rule.id) (extra)\n    \(rule.summary)") }
    exit(0)
}

var paths: [String] = []
var enabled: Set<String> = []
for argument in arguments {
    if argument.hasPrefix("--enable=") {
        enabled.formUnion(argument.dropFirst("--enable=".count).split(separator: ",").map(String.init))
    } else if argument.hasPrefix("-") {
        fail("unknown option: \(argument)\n\(usage)")
    } else {
        paths.append(argument)
    }
}
if paths.isEmpty { fail(usage) }
let unknown = enabled.subtracting(SlopRule.extraRules.map { $0.id })
if !unknown.isEmpty { fail("no extra rule \(unknown.sorted().joined(separator: ", "))") }

let rules = SlopRule.allRules + SlopRule.extraRules.filter { enabled.contains($0.id) }
let files = paths.flatMap(swiftFiles)
var violations: [Violation] = []
for file in files {
    guard let source = try? String(contentsOfFile: file, encoding: .utf8) else {
        fail("\(file): not UTF-8")
    }
    violations += AntiSlop.lint(source: source, fileName: file, rules: rules)
}
violations.sort { ($0.fileName, $0.line, $0.column) < ($1.fileName, $1.line, $1.column) }
for violation in violations {
    print("\(violation.fileName):\(violation.line):\(violation.column): error: \(violation.description)")
}
print("[anti-slop] \(violations.count) violations across \(files.count) files (\(rules.count) rules).")
exit(violations.isEmpty ? 0 : 1)
