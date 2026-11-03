# Tessera daemon

Rust companion for the Tessera desk display. The installed binary is named **Tessera**, and its LaunchAgent label is `com.tessera.agentd`. The desktop UI is in [`apps/agent-ui`](../apps/agent-ui/).

Start with [Agent development](../docs/guides/agent-v2-dev.md) for toolchains, build/test/install commands and release instructions.

## Responsibilities

- Detect and connect to the panel over USB CDC.
- Collect Mac music metadata, native calendars, Focus status, AI quotas and Todoist content.
- Serve localhost IPC to the desktop UI, including setup, settings, diagnostics and updates.
- Keep credentials on the Mac and send only selected content snapshots to the panel.

Capabilities own independent workers. Network requests, Keychain, AppleScript and artwork decoding must not block serial heartbeats or music updates. Bounded command queues prevent unbounded work when a provider is slow.

## Calendar

The source is **Calendar on macOS**, through EventKit. iCloud, Google, Exchange and other accounts configured there are available after native permission. Tessera only reads selected calendars. It does not import accounts, run Google OAuth, or edit events.

The build embeds a signed helper with Calendar privacy descriptions. An explicit permission action launches **Tessera Calendar** through Launch Services; routine scans never ask for permission. See [Calendar setup](../docs/guides/calendar-setup.md).

## Music and Focus

Music is informational: no playback commands. `MEDIA\tv1` preserves its legacy prefix and optionally adds trailing `album`, `year` and `cover_key` fields. The current player supplies metadata; Spotify's scripting interface does not supply a release year.

Artwork is converted to 200×200 RGB565, cached, chunked and bound to the track key. The firmware acknowledges complete reception with `MEDIA_COVER_OK\t<key>`; missing acknowledgment triggers retry. Old-track and incomplete transfers must never become a complete image.

Focus is observational: the daemon reads actual Mac state. Unsupported private database writes and optimistic success states have been removed. Change Focus in macOS Control Center.

## AI usage

Local credentials resolve the enabled providers. Cursor and Codex use live usage requests; Claude reads its persistent local usage cache, including the weekly quota window. Transient failures retain the last verified values for up to six hours in process memory, then become unavailable rather than fabricated percentages.

## Protocol and IPC

USB payloads are tab-separated lines; encoders exclude the transport newline. Legacy golden fixtures protect compatible prefixes. They are historical fixtures and do not require the retired Python runtime.

The UI talks to `127.0.0.1:19792` (override: `TESSERA_AGENTD_IPC_PORT`). The live API requires a per-installation Bearer token and accepts only the native Tauri origins. Browser tests use an isolated synthetic server on port 19793. See the [full IPC contract](../apps/agent-ui/IPC.md).

The token stays in `~/.tessera/agentd/ipc_token` (mode 0600). The main window obtains it through its scoped `ipc_token` Tauri permission. Never include it in browser URLs, screenshots or issue reports.

## Offline distribution

`make pack-agentd` builds a fallback SD/portal package with the daemon and app. Packages must not contain credentials, tokens, `.env` or local configuration. The normal installation path is the public site's complete companion installer.
