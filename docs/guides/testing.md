# Testing user journeys

A green unit suite does not prove that a user can finish setup or navigate the panel. Tessera has three complementary journey layers. Generated screenshots and logs stay under `/tmp`, outside Git.

## Firmware, without hardware

```sh
make test
make e2e
```

`make e2e` builds the actual firmware screens on SDL, captures offline/loading/error scenarios, then runs `simulator/journeys/*.txt`. These journeys inject pointer press/release and drag input into LVGL, just as a user would, and assert visible text and navigation. They do not call navigation functions to fake a successful click.

Current journeys cover home → weather → home, task scrolling, settings, and music track changes, album/year, pause, stop and back. The fixture data is synthetic. Art transfer validation also has native tests for complete reception, missing/out-of-order chunks and stale-track rejection.

Inspect `/tmp/tessera-sim-out/journey-*.bmp` and logs after UI changes. The simulator cannot validate the physical touch controller, USB buffering, display DMA or macOS permission dialogs.

## Agent and public site, in a browser

Install the site's locked dependencies and build it, then start the isolated server:

```sh
npm ci --prefix site
npm run build --prefix site
python3 tools/e2e/serve.py
# In another terminal, with browser-harness installed and Chrome debugging enabled:
make e2e-browser
```

The server binds to loopback only: UI/site on port 1420 and **synthetic IPC on 19793**. It never talks to the installed daemon on 19792. Only the served copy of `ipc.js` has its port changed; the product ships no mock mode.

Browser journeys use accessible names, pointer clicks and assertions. They cover first-use, actual USB gating, persisted setup, calendar permission/selection/denial, network failure, daemon failure, responsive layouts, site introduction, installation links and version manifest. Screenshots are saved in `/tmp/tessera-browser-e2e`.

These tests validate the web UI and its HTTP contract against fixtures. They do not prove native Tauri window behavior, actual EventKit permission grants, real provider changes or release installation. Revalidate those separately on a Mac.

## Physical device

After the host gate is green:

```sh
make flash
make e2e-device
```

The device runner pauses only `com.tessera.agentd`, opens the panel, and restores the service in `finally`. It requires a 320×480 portrait panel with weather and music visible. It uses `UI_TREE` to find visible touch targets, sends `DRAG` through the same input injection path as a touch, then asserts the resulting UI tree.

Music uses synthetic snapshots in RAM. Tests deliberately omit an artwork chunk, check that no completion is acknowledged, retry the complete image at the production 20 ms packet pacing with CRC, switch tracks, reject the previous cover, and verify the next image's acknowledgment. No accounts, tokens, Wi‑Fi or device preferences are changed. Weather navigation accepts loading/error states; a live forecast is not a prerequisite for checking the detail view and back action. The installed Agent resumes real content afterward.

Artifacts go to `/tmp/tessera-device-e2e`. Inspect captures and `result.json`; `PERF` is recorded for comparison. This automation tests actual USB/parser/display paths but does not create real player or Calendar events.

## Native Mac integration

Run `cargo test --workspace` in `agentd/`. Build the app, install the companion and test the real Calendar permission: allow, deny, return from System Settings, select calendars and reconnect USB. Confirm a calendar already configured in macOS appears without another login. Native consent remains a deliberate human action; automation must not grant permissions behind the user's back.

For music, change tracks in the actual player and inspect metadata/art on the panel. Unsupported or missing metadata must remain absent, not guessed. Tests with synthetic events and explicit error scenarios complement this real-source check.

The physical UI inspector selects the frontmost full-screen opaque layer. Detail/settings pages live above the home screen, so querying only `lv_screen_active()` would incorrectly assert against hidden home content. Scripted input starts its clock at the first LVGL sample and releases at the final gesture coordinates, preventing rendering delays from swallowing a short injected touch.

## Screensaver cadence

`make e2e` also runs the real screensaver code with controlled LVGL and wall clocks. It interrupts an active flip for three seconds, crosses midnight, applies a backward time correction, and changes between HH:MM:SS and HH:MM. A resumed frame must immediately show current time with no old animation pending.

With the panel connected and seconds already enabled, run `make e2e-clock`. It measures 65 seconds of clock updates while repeatedly transferring valid synthetic artwork over USB at approximately 20 KiB/s. It asserts a UI sampling gap below 250 ms, at least 62 wall-time updates, and successful cover acknowledgments. It pauses and restores the companion, writes artifacts to `/tmp/tessera-clock-e2e`, and leaves preferences unchanged. `REST stats` reports maximum sampling gap, time changes, resynchronizations, active flips and settled digits.
