import AppKit
// apps-named <표시 이름>: 표시 이름(Dock 타일 이름)이 같은 실행 중 앱의 pid를 공백으로 출력한다(읽기 전용).
let name = CommandLine.arguments[1]
let pids = NSWorkspace.shared.runningApplications.filter { $0.localizedName == name }.map { String($0.processIdentifier) }
print(pids.joined(separator: " "))
