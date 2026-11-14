// Compares measured control surfaces and the settings page across native interaction states.
import AppKit
import Foundation

let directory = URL(fileURLWithPath: CommandLine.arguments[1])
let files = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
func bitmap(_ url: URL) throws -> NSBitmapImageRep {
    guard let rep = NSBitmapImageRep(data: try Data(contentsOf: url)) else { fatalError("Invalid PNG") }
    return rep
}
func differences(_ baseline: NSBitmapImageRep, _ other: NSBitmapImageRep, rows: Range<Int>) -> Int {
    precondition(other.pixelsWide == baseline.pixelsWide && other.pixelsHigh == baseline.pixelsHigh)
    precondition(!rows.isEmpty && rows.upperBound <= baseline.pixelsHigh)
    var changed = 0
    for y in rows {
        for x in 0..<baseline.pixelsWide {
            if baseline.colorAt(x: x, y: y) != other.colorAt(x: x, y: y) { changed += 1 }
        }
    }
    return changed
}
var allResults: [String: [String: Int]] = [:]
for prefix in ["interaction", "appkit", "settings"] {
    guard let before = files.first(where: { $0.lastPathComponent.hasPrefix(prefix + "-") && $0.lastPathComponent.hasSuffix("-before.png") }) else { fatalError("Missing \(prefix) baseline") }
    let boundary = Double(before.lastPathComponent.split(separator: "-")[1])!
    let baseline = try bitmap(before)
    // Viewport widths are fixed inputs of the corresponding acceptance fixtures.
    let scale = Double(baseline.pixelsWide) / (prefix == "settings" ? 1200 : 900)
    let rows = prefix == "settings" ? Int(ceil(boundary * scale))..<baseline.pixelsHigh : 0..<Int(boundary * scale)
    var results: [String: Int] = [:]
    for state in prefix == "settings" ? ["blocked"] : ["blocked", "restored", "unavailable"] {
        let other = try bitmap(directory.appendingPathComponent(before.lastPathComponent.replacingOccurrences(of: "-before.png", with: "-\(state).png")))
        results[state] = differences(baseline, other, rows: rows)
    }
    allResults[prefix] = results
}
let data = try JSONSerialization.data(withJSONObject: allResults, options: [.prettyPrinted, .sortedKeys])
try data.write(to: directory.appendingPathComponent("pixel-comparison.json"))
print(String(data: data, encoding: .utf8)!)
for (prefix, results) in allResults {
    if results["blocked"] != 0 { exit(1) }
    if prefix != "settings" && (results["restored"] != 0 || results["unavailable"] == 0) { exit(1) }
}
