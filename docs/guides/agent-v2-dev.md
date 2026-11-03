# Develop the Mac Agent

The companion consists of `agentd/` (Rust daemon) and `apps/agent-ui/` (Tauri app). The Python companion has been retired.

## Toolchain

Install Xcode command line tools for the native Calendar reader (`xcode-select --install`). Review `mise.toml`, then:

```sh
mise trust
mise install
mise run agent-check-trees
```

The pinned toolchain is Rust 1.90 and Node 22. Native Calendars require macOS 13 or later. Non-Mac hosts can work on codecs and firmware; they cannot collect native Calendar data or build the Mac app.

## Build and test

```sh
mise run agentd-test
mise run agentd-build
mise run agent-ui-build
```

Equivalent direct commands, useful when mise has not been trusted yet:

```sh
cargo test --manifest-path agentd/Cargo.toml --workspace
cargo build --manifest-path agentd/Cargo.toml --release -p agentd
npm ci --prefix apps/agent-ui
npm --prefix apps/agent-ui run tauri -- build --bundles app
```

For browser and real-panel user journeys, see [testing](testing.md). An opt-in real-player check reads the currently playing track without changing playback:

```sh
cargo test --manifest-path agentd/Cargo.toml -p tessera-caps-media --test live_media -- --ignored
```

## Install a local build

```sh
make agent-install
```

This builds both components, installs `/Applications/Tessera.app` and the `com.tessera.agentd` LaunchAgent, and restarts the service. It changes the installed companion on this Mac and pauses automatic updates so a public release cannot immediately overwrite the local build. Re-enable updates from Help/app after publishing/installing a compatible release. Ordinary tests do not change these preferences.

To run a foreground daemon, stop the installed LaunchAgent first so two processes cannot own the USB port. Restore the LaunchAgent when finished. Never leave a second daemon running as a debugging shortcut.

## Interfaces

The UI uses localhost IPC on port 19792; see [IPC contract](../../apps/agent-ui/IPC.md). Native Calendar selection works directly through IPC even without a panel. Device settings use the panel's Wi‑Fi portal; USB carries Mac snapshots.

Each capability runs on its own worker. Do not introduce network requests or artwork processing into the serial loop. MEDIA metadata must be sent before artwork, and late artwork must match the current track identity.

## Release and site

A tag `agent-vX.Y.Z` runs `agent-release.yml`, tests/builds the companion, publishes its artifacts and updates the Pages manifest. The workflow aligns the actual daemon/app version with the tag so self-update does not repeatedly install a binary that reports an older version.

Publish the companion version that supports a new setup flow before making that flow the public installation guide. See [site development](../../site/README.md). Notarization and a verified Intel release remain separate distribution work.

The firmware finish gate remains `make test` → `make e2e` → `make flash` → `make device` (or repeatable device journeys).
