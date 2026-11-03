# Repository assets

Version material required to build, understand or test the product. Keep generated session output local.

## Keep in Git

- Firmware, simulator, Agent, portal and site sources; lockfiles and build scripts.
- Brand master SVGs and the icon derivatives needed by release builds.
- Deterministic, sanitized simulator fixtures and selected screenshots used by documentation or the site.
- User/contributor guides, architecture decisions, and useful design history.

## Keep outside Git

- Device captures containing real task titles, calendar names, personal accounts, Wi‑Fi details or live usage data. Existing local `device-*.png` captures are ignored; the reviewed first-use capture remains tracked.
- `docs/social/`: video editor session caches, thumbnails, previews, captured personal data and downloaded audio. These are not inputs to the product build. Keep the local working directory; do not delete it as cleanup.
- `docs/media/`: rendered promotional videos. Publish reviewed, sanitized final videos as release/media attachments when needed, rather than storing every render in the product trunk.
- Build directories, `site/dist/`, test screenshots/logs, local settings and credentials.

The site should use curated product screenshots, not raw device captures. A media contribution must identify its license and use synthetic content. If reproducible promotional source becomes a maintained project, move reviewed source to a dedicated package with explicit assets and attribution; do not add an entire editor cache.
