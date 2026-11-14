// Posts only configured test chords through macOS input, with explicit permission and background checks.
import AppKit
import CoreGraphics
import Foundation

func finish(_ result: [String: Any], _ path: String, _ code: Int32) -> Never {
    do { try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys]).write(to: URL(fileURLWithPath: path), options: .atomic) }
    catch { fputs("Cannot write input receipt: \(error)\n", stderr) }
    exit(code)
}
let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 2 else { exit(64) }
if args[0] == "--check" || args[0] == "--request" {
    let permitted = args[0] == "--request" ? CGRequestPostEventAccess() : CGPreflightPostEventAccess()
    finish(["permitted": permitted], args[1], permitted ? 0 : 2)
}
guard args.count == 5, args[0] == "--send", let target = Int32(args[3]) else { exit(64) }
let receipt = args[4]
guard CGPreflightPostEventAccess() else { finish(["error": "input_permission_required"], receipt, 2) }
do {
    let config = try JSONSerialization.jsonObject(with: Data(contentsOf: URL(fileURLWithPath: args[1]))) as! [String: Any]
    let input = config["input"] as! [String: Any]
    let keys = input["keys"] as! [String: [String: Any]]
    guard let chord = keys[args[2]], let key = chord["keyCode"] as? UInt16 else {
        finish(["error": "unknown_test_key"], receipt, 64)
    }
    let timeout = input["timeoutMs"] as! Double / 1000
    let poll = input["pollMs"] as! Double / 1000
    let deadline = Date().addingTimeInterval(timeout)
    while NSWorkspace.shared.frontmostApplication?.processIdentifier == target && Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(poll))
    }
    guard kill(target, 0) == 0, NSWorkspace.shared.frontmostApplication?.processIdentifier != target else {
        finish(["error": "target_not_background"], receipt, 1)
    }
    let modifiers = input["modifiers"] as! [[String: NSNumber]]
    var flags = CGEventFlags()
    let source = CGEventSource(stateID: .privateState)
    func post(_ code: UInt16, _ down: Bool, _ eventFlags: CGEventFlags) throws {
        guard let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down) else {
            throw NSError(domain: "shortcut_input", code: 1)
        }
        event.flags = eventFlags
        event.post(tap: .cghidEventTap)
    }
    for modifier in modifiers {
        flags.insert(CGEventFlags(rawValue: modifier["flag"]!.uint64Value))
        try post(modifier["keyCode"]!.uint16Value, true, flags)
    }
    try post(key, true, flags)
    try post(key, false, flags)
    for modifier in modifiers.reversed() {
        flags.remove(CGEventFlags(rawValue: modifier["flag"]!.uint64Value))
        try post(modifier["keyCode"]!.uint16Value, false, flags)
    }
    finish(["posted": true, "wasBackground": true, "key": args[2]], receipt, 0)
} catch {
    finish(["error": String(describing: error)], receipt, 1)
}
