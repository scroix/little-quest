import CoreGraphics
import Foundation

// Prints the CGWindowID of the first on-screen, layer-0 window owned by the given pid.
guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]) else {
    FileHandle.standardError.write("usage: window_id <pid>\n".data(using: .utf8)!)
    exit(2)
}
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
for w in list {
    guard let owner = w[kCGWindowOwnerPID as String] as? Int32, owner == pid,
          let layer = w[kCGWindowLayer as String] as? Int, layer == 0,
          let id = w[kCGWindowNumber as String] as? Int
    else { continue }
    print(id)
    exit(0)
}
exit(1)
