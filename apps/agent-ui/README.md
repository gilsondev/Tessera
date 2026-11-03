# Tessera Mac app

The Tauri companion UI handles first-use setup, native Calendar permission/selection, Mac integrations, device preferences, diagnostics and updates. It uses the Rust daemon's localhost IPC; it never substitutes demo data when the daemon is unavailable.

```sh
npm ci
npm run tauri dev
npm run tauri build -- --bundles app
```

Run these commands in this directory. For the full toolchain and installed companion, see [Agent development](../../docs/guides/agent-v2-dev.md). Native macOS development requires Xcode command line tools.

- [IPC contract](IPC.md)
- [Automated browser and device journeys](../../docs/guides/testing.md)
- [Which settings belong on each surface](../../docs/guides/settings-surfaces.md)
- [Product journey decisions](../../docs/guides/product-consolidation.md)

The browser test server serves these actual frontend sources with isolated synthetic IPC on port 19793. The installed app uses port 19792. There is no mock switch in the product.
