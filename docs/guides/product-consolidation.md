# Product consolidation review

This is a review of the current user journeys, not an unchecked historical feature plan.

## Decisions

- **Calendar source:** the calendars already configured in macOS. Native EventKit permission replaces account import and Google-specific OAuth. The Agent owns permission and calendar selection; the panel receives selected event snapshots only.
- **Music:** informational, with artwork, track, artist, album and year when supplied. Playback actions are removed. Metadata must not wait for art or unrelated providers.
- **Focus:** informational. Unsupported private database writes, shortcut guesses and optimistic success states are removed. Users change Focus in macOS Control Center.
- **Settings:** device preferences have one source of truth on the panel. Text-heavy setup lives in the Agent/web portal; quick physical adjustments stay on the device. The Agent clearly distinguishes USB synchronization from Wi‑Fi configuration.

## User journey changes

The public site now introduces the device and Agent before asking the visitor to install anything. The start guide separates a new board (developer flashing required) from an existing Tessera, explains the complete companion installer, identifies current Mac support and links directly to contribution instructions.

The Agent's primary navigation is reduced to Home, Panel, Widgets, Integrations and Help/app. Machine profiles and technical activity are secondary. First use hides daily-use tabs and follows discovery → choices → permissions → real USB detection, with an explicit option to connect later. A fake “simulate found” action and demo fallback data no longer ship.

Calendar setup works without the device: allow native access, choose local calendars, then connect USB. Granted permission is summarized; denied access points to the correct System Settings pane. The app never asks for account JSON or another Google login.

Widget display preferences and Mac collection choices are separate: displaying a provider on the panel must not silently change what the Mac collects. Offline device settings explain the missing Wi‑Fi connection.

## Verification boundaries

- Native tests cover domain parsing and transfer integrity.
- Simulator journeys cover actual pointer input, navigation, scrolling and track-state changes.
- Browser journeys cover first-use, persisted setup, native-calendar UI states against synthetic IPC, offline failures and the site's first visit at desktop/narrow widths.
- Physical journeys cover real USB, touch injection, UI assertions and artwork loss/retry/identity checks.
- After the human granted EventKit access, native status and snapshot reads were verified against the real Mac catalog (13 calendars). The installed UI showed the permitted state and all 13 choices. Selection and event delivery require the user to choose calendars. A mock grant is not proof of a real system permission. No test may silently grant system permissions.
- A live music read verified current title, artist, album and decodable artwork without changing playback. Actual player track changes remain a separate source check; physical artwork retry and track identity were validated with synthetic data.

## Remaining distribution considerations

The app is not notarized and the existing release workflow builds Apple Silicon. A public firmware installer and verified Intel release are separate deliverables. Spotify's current scripting API does not supply a release year; do not guess it or display a false value. Missing/provider-withheld artwork needs a stable placeholder, while transfer failures are retried.

Raw personal captures and video editor sessions stay local. Only curated, synthetic product imagery is included in the site and docs; see [asset policy](repository-assets.md).

## Control responsiveness

Switches use a 44×26 px visual track with a separate expanded hit area; generic button/input minimums must not resize their track or thumb. Discovery badges size to their text instead of stretching across their parent. Clicks update immediately, saves are serialized per control, and the final intent wins if the user clicks again while a save is pending. In-flight polling cannot replace that intent with older state.

Saving a capability edits the cached discovery result; only an explicit discovery request scans apps, credentials and native Calendar access. Collection choices never silently change device display preferences. Browser tests deliberately delay saves to verify responsiveness and repeated clicks.

## Local validation — 2026-10-28

Firmware native suite: 116 passed. Rust workspace: 76 passed, two opt-in integrations ignored; the live media integration was also run explicitly and passed. Simulator scenarios and pointer journeys passed. Browser journeys passed, including delayed saves, repeated clicks and compact control dimensions. Seven physical USB/touch/artwork checks passed after flashing the verified firmware. The final native Agent was built and installed locally; automatic updates were paused to prevent the public release from replacing this development build.

The site builds locally. These changes have not been published as a release or deployed to Pages. Distribution should publish a matching companion build before directing new users through the updated public setup journey.

### Calendar permission follow-up

A direct helper subprocess inherited the daemon's TCC responsibility, so status reads incorrectly returned `not_determined` after the user had authorized the helper app. All commands now launch the same signed helper through Launch Services. A private temporary output file carries the JSON response; completion is based on valid output because `open -W` can return early for a short-lived executable. The existing helper signature and granted permission are preserved. An explicit, read-only native integration test covers repeated authorized status and snapshot reads.

## Screensaver follow-up — 2026-11-03

Under USB load, clock sampling gaps reached 771 ms. Agent heartbeats rebuilt the portal snapshot, whose resource sampler called Arduino `ESP.getSketchSize()` each time. That API calls `esp_image_verify()` across the running image. The image size is now cached for the boot, and unchanged heartbeats only refresh their RAM timestamp.

USB input yields within a bounded byte/time budget so continuous transfers cannot monopolize the UI loop. The hardware CDC receive buffer is increased from its 256-byte default to 16 KiB so packets arriving during a rendered frame are retained instead of dropped. Flip frames use font metrics without forcing a full layout at every animation step. If the UI resumes late or wall time changes discontinuously, the screensaver settles directly on current time and cancels obsolete animations instead of replaying queued seconds. Simulator regression tests cover interrupted flips, midnight and backward time corrections.

The consolidation branch starts from this workspace's original base. Remote `main` advanced by 62 commits independently; pushing the branch preserves those changes and does not merge over them. Integration with that newer trunk needs a separate conflict review.

Final physical validation: 65 seconds at approximately 60 KiB/s of synthetic artwork traffic yielded a maximum UI sampling gap of 96 ms, 68 wall-time updates, no discontinuity resynchronizations, and 17 complete artwork acknowledgments. The seven touch/media integrity checks passed. Native tests (116), simulator scenarios/timing regression, Rust workspace (76), and the site build were green. These are local-device measurements, not a guaranteed frame-rate bound for arbitrary USB flooding or blocking maintenance operations.
