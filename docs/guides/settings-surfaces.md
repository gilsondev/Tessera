# Where to configure Tessera

Each setting has one owner. The Agent and web portal edit the same device preferences; they do not keep competing local copies.

| Task | Device | Local web portal | Mac Agent | Owner |
| --- | --- | --- | --- | --- |
| Brightness, appearance, orientation, screen rest | Quick changes | Full controls | Panel tab, over Wi‑Fi | Device |
| Show/hide widgets, tile height, format | Quick widget settings | Full controls | Panel / Widgets, over Wi‑Fi | Device |
| Wi‑Fi setup and recovery | Connection information | Captive setup / network configuration | Link to local portal | Device |
| City and other text-heavy preferences | Summary / guidance | Full controls | Widgets, over Wi‑Fi | Device |
| Calendar permission and selection | Status / guidance | Status and request for permission on Mac | Integrations → Agenda | Mac |
| AI credentials and collection permissions | Status | Display preferences only | Integrations | Mac |
| Music and Focus | Current state | Display enablement | Collection / diagnostics | Mac, read-only |
| Agent updates, permissions, logs | Connection status | Installation link | Help and app | Mac |

**USB** transports snapshots from the Mac. **Wi‑Fi** is required for the Agent to edit device preferences through the local portal. Offline controls must say which connection is missing and must not claim a setting was saved.

Avoid exposing technical diagnostics in normal configuration. Logs, version numbers, permissions and troubleshooting belong under Help/Diagnostics. Do not add playback or Focus action buttons: these integrations are informational by product decision.
