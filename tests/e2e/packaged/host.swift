// Inspects LaunchServices and controls only the uniquely identified acceptance application.
import AppKit
import CoreServices
import CoreGraphics
import Foundation

func emit(_ value: Any) throws {
    let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    print(String(decoding: data, as: UTF8.self))
}
let args = Array(CommandLine.arguments.dropFirst())
guard let command = args.first else { fatalError("missing command") }
switch command {
case "session":
    let session = CGSessionCopyCurrentDictionary() as? [String: Any] ?? [:]
    let locked = session["CGSSessionScreenIsLocked"] as? Bool ?? false
    let onConsole = session[kCGSessionOnConsoleKey as String] as? Bool ?? false
    let front = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
    try emit(["locked": locked, "onConsole": onConsole,
              "frontmost": front as Any? ?? NSNull(),
              "ready": onConsole && !locked && front != nil && front != "com.apple.loginwindow"])
case "directories":
    let manager = FileManager.default
    try emit([
        "data": manager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].path,
        "cache": manager.urls(for: .cachesDirectory, in: .userDomainMask)[0].path,
        "library": manager.urls(for: .libraryDirectory, in: .userDomainMask)[0].path,
    ])
case "register":
    let result = LSRegisterURL(URL(fileURLWithPath: args[1]) as CFURL, true)
    guard result == noErr else { fatalError("LSRegisterURL failed: \(result)") }
    try emit(["registered": true])
case "handlers":
    let url = URL(string: args[2])!
    let urls = NSWorkspace.shared.urlsForApplications(toOpen: url)
    let handlers = urls.compactMap { Bundle(url: $0)?.bundleIdentifier }
    let selected = NSWorkspace.shared.urlForApplication(toOpen: url).flatMap { Bundle(url: $0)?.bundleIdentifier }
    try emit(["handlers": handlers, "default": selected as Any? ?? NSNull(), "applications": urls.map { $0.path }])
case "restore":
    let status = LSSetDefaultHandlerForURLScheme(args[1] as CFString, args[2] as CFString)
    guard status == noErr else { fatalError("cannot restore original URL handler: \(status)") }
    try emit(["restored": true])
case "process", "hide", "terminate", "force":
    let apps = NSRunningApplication.runningApplications(withBundleIdentifier: args[1])
    if command == "hide" { apps.forEach { _ = $0.hide() } }
    if command == "terminate" { apps.forEach { _ = $0.terminate() } }
    if command == "force" { apps.forEach { _ = $0.forceTerminate() } }
    try emit(apps.map { ["pid": Int($0.processIdentifier), "hidden": $0.isHidden,
                        "active": $0.isActive, "terminated": $0.isTerminated] as [String: Any] })
default: fatalError("unsupported command")
}
