#!/usr/bin/env bash
# Build and verify the actual JC3248 upload; always restore the companion.
set -euo pipefail
cd "$(dirname "$0")/.."
UID_NUM="$(id -u)"
AGENT_PLIST="$HOME/Library/LaunchAgents/com.tessera.agentd.plist"
AGENT_SERVICE="gui/$UID_NUM/com.tessera.agentd"
AGENT_WAS_RUNNING=0
UPLOAD_LOG="$(mktemp /tmp/tessera-upload.XXXXXX)"
resume_agent() {
  if [ "$AGENT_WAS_RUNNING" = 1 ] && [ -f "$AGENT_PLIST" ]; then
    launchctl bootstrap "gui/$UID_NUM" "$AGENT_PLIST" 2>/dev/null || true
    launchctl kickstart -k "$AGENT_SERVICE" 2>/dev/null || true
  fi
  rm -f "$UPLOAD_LOG"
}
trap resume_agent EXIT
if launchctl print "$AGENT_SERVICE" >/dev/null 2>&1; then
  AGENT_WAS_RUNNING=1
  launchctl bootout "gui/$UID_NUM" "$AGENT_PLIST"
fi
sleep 0.5
.venv/bin/pio run -e jc3248w535
.venv/bin/python - <<'PY'
import glob, time, serial, termios
ports = sorted(p for p in glob.glob('/dev/cu.usbmodem*') if not p.endswith('1401'))
if ports:
    try:
        with serial.Serial(ports[0],115200,timeout=.2) as s:
            s.dtr=True;s.write(b'FLASH\n');s.flush()
            deadline=time.monotonic()+1.5
            while time.monotonic()<deadline and b'flash pronto' not in s.read(256): pass
    except (serial.SerialException,OSError,termios.error): pass
    try:
        with serial.Serial(ports[0],1200) as s:
            s.dtr=False;s.rts=False;time.sleep(.4)
    except (serial.SerialException,OSError,termios.error): pass
for _ in range(60):
    if glob.glob('/dev/cu.usbmodem1401'): break
    time.sleep(.15)
else: raise SystemExit('JC3248 bootloader did not appear; upload was not performed')
PY
.venv/bin/pio run -e jc3248w535 -t upload --upload-port /dev/cu.usbmodem1401 | tee "$UPLOAD_LOG"
if ! grep -q 'Hash of data verified' "$UPLOAD_LOG"; then
  echo 'Upload did not confirm the flash hash; refusing to report success.' >&2
  exit 1
fi
