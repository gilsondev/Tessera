# Calendars from your Mac

Tessera reads the calendars already added to **Calendar on macOS**. This includes iCloud, Google, Exchange, CalDAV and local calendars available in that app. You do not need a Google Cloud project, OAuth JSON, account import or another login.

1. Open **Tessera → Integrations → Configure Agenda**.
2. Select **Allow Calendars** and confirm the native macOS permission for **Tessera Calendar**.
3. Select the calendars you want on the panel. The selection saves automatically on this Mac.
4. Connect the panel by USB. Upcoming events appear after the next snapshot.

macOS calls its EventKit reading permission **full access**. Tessera's implementation only reads events; it does not create, edit or delete them.

## Troubleshooting

- **Permission denied:** open System Settings → Privacy & Security → Calendars and allow Tessera Calendar. Return to the Agent; it checks again automatically.
- **Account missing:** open Calendar → Settings → Accounts on the Mac, add or enable the account, and wait for Calendar to sync. Tessera does not manage these accounts.
- **Calendar missing:** confirm it is available in the Mac Calendar app. Reopen the Agent's Agenda screen.
- **No events on the panel:** select at least one calendar, check the USB connection and confirm there are upcoming events in the next eight days.
- **Panel disconnected:** you can still grant permission and choose calendars; USB is only needed to send snapshots to the display.

## For contributors

`agentd/crates/caps-calendar/native/Calendar.swift` uses Apple's EventKit API. The Rust build compiles and embeds a helper with Calendar privacy descriptions. Xcode command line tools (`xcode-select --install`) are required for macOS builds. Other platforms retain the USB codec but cannot collect Mac calendars.

Selections live in `~/.tessera/calendar/selection.json`. Existing Google OAuth files from earlier versions are left untouched but are no longer used. Local calendar identifiers differ from the legacy Google identifiers, so select calendars again after upgrading.

The helper does not request permission during scans or routine collection. Only the explicit **Allow Calendars** action starts a native request. Denied/restricted/unavailable states must stay visible and must never be replaced with demo events.

Reference: [Apple EventKit access](https://developer.apple.com/documentation/eventkit/accessing-calendar-using-eventkit-and-eventkitui).
