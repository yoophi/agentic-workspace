import AppKit
// apps-named <표시 이름> | apps-named --bundle <번들 id>: 표시 이름(Dock 타일 이름) 또는 번들 id가 같은 실행 중 앱의 pid를
// 공백으로 출력한다(읽기 전용).
let args = CommandLine.arguments
let apps: [NSRunningApplication]
if args.count > 2 && args[1] == "--bundle" {
  apps = NSRunningApplication.runningApplications(withBundleIdentifier: args[2])
} else {
  apps = NSWorkspace.shared.runningApplications.filter { $0.localizedName == args[1] }
}
print(apps.map { String($0.processIdentifier) }.joined(separator: " "))
