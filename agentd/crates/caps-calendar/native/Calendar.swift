import Foundation
import EventKit
import AppKit

// A standalone, signed EventKit reader. Never mutates calendar data.
let store = EKEventStore()
let command = CommandLine.arguments.dropFirst().first ?? "status"
func permission() -> String {
    let state = EKEventStore.authorizationStatus(for: .event)
    if #available(macOS 14.0, *), state == .fullAccess { return "authorized" }
    switch state {
    case .authorized: return "authorized"
    case .denied: return "denied"
    case .restricted: return "restricted"
    case .notDetermined: return "not_determined"
    default: return "denied"
    }
}
func color(_ cal: EKCalendar) -> String {
    guard let cg = cal.cgColor, let c = NSColor(cgColor: cg)?.usingColorSpace(.deviceRGB) else { return "1A73E8" }
    return String(format: "%02X%02X%02X", Int(c.redComponent * 255), Int(c.greenComponent * 255), Int(c.blueComponent * 255))
}
func emit() {
    let allowed = permission() == "authorized"
    let calendars = allowed ? store.calendars(for: .event) : []
    let selected = Set((CommandLine.arguments.count > 2 ? CommandLine.arguments[2].data(using: .utf8).flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String] } : nil) ?? [])
    var events: [[String: Any]] = []
    if allowed && command == "snapshot" && !selected.isEmpty {
        let now = Date()
        let calendar = Calendar.current
        let start = calendar.startOfDay(for: now)
        let end = calendar.date(byAdding: .day, value: 8, to: start)!
        let chosen = calendars.filter { selected.contains($0.calendarIdentifier) }
        if !chosen.isEmpty {
            let predicate = store.predicateForEvents(withStart: start, end: end, calendars: chosen)
            let formatter = DateFormatter(); formatter.dateFormat = "HHmm"
            events = store.events(matching: predicate).filter { $0.endDate > now }.sorted {
                let a = calendar.startOfDay(for: max($0.startDate, start)), b = calendar.startOfDay(for: max($1.startDate, start))
                if a != b { return a < b }
                if $0.isAllDay != $1.isAllDay { return $0.isAllDay }
                return $0.startDate < $1.startDate
            }.prefix(12).map { e in [
                "start_hhmm": formatter.string(from: e.startDate), "end_hhmm": formatter.string(from: e.endDate),
                "all_day": e.isAllDay, "title": e.title ?? "(sem título)", "location": e.location ?? "",
                "duration_min": max(1, Int(e.endDate.timeIntervalSince(e.startDate) / 60)), "color": color(e.calendar),
                "day_offset": max(0, calendar.dateComponents([.day], from: start, to: calendar.startOfDay(for: e.startDate)).day ?? 0)
            ] }
        }
    }
    let result: [String: Any] = ["permission": permission(), "linked": allowed, "calendars": calendars.map { c in
        ["id": c.calendarIdentifier, "name": c.title, "account": c.source.title, "color": color(c), "selected": selected.contains(c.calendarIdentifier)] as [String: Any]
    }, "events": events, "fetched_at": ISO8601DateFormatter().string(from: Date())]
    let data = try! JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
    FileHandle.standardOutput.write(data)
}
if command == "connect" && permission() == "not_determined" {
    var done = false
    let completion: (Bool, Error?) -> Void = { _, _ in DispatchQueue.main.async { done = true } }
    if #available(macOS 14.0, *) { store.requestFullAccessToEvents(completion: completion) }
    else { store.requestAccess(to: .event, completion: completion) }
    let deadline = Date().addingTimeInterval(170)
    while !done && Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.05)) }
}
emit()
