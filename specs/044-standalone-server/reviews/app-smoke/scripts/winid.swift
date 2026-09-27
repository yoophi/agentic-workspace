import CoreGraphics
import Foundation
// winid <pid>: 그 프로세스의 화면 위 창 id(layer 0)를 출력한다.
let pid = Int32(CommandLine.arguments[1])!
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as! [[String: Any]]
for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
  print(w[kCGWindowNumber as String]!, w[kCGWindowName as String] ?? "")
}
