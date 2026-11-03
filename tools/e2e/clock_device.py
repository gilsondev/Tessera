"""Measure screensaver cadence under USB artwork load; restore agent in finally.

Synthetic music lives only in RAM. No preferences or calendar/task data change.
Requires seconds enabled in the clock's existing preferences.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from panel_probe import open_port, command, snapshot

OUT = Path('/tmp/tessera-clock-e2e')

def main():
    OUT.mkdir(exist_ok=True)
    service = f'gui/{os.getuid()}/com.tessera.agentd'
    plist = str(Path.home()/'Library/LaunchAgents/com.tessera.agentd.plist')
    running = subprocess.run(['launchctl', 'print', service], capture_output=True).returncode == 0
    ser = None
    log = ''
    try:
        if running:
            subprocess.run(['launchctl', 'bootout', f'gui/{os.getuid()}', plist], check=True, capture_output=True)
        ser = open_port()
        ser.write_timeout = 2
        # Freshly flashed boards can still be mounting storage and building UI.
        # Wait for a real console response before measuring or sending fixtures.
        deadline = time.monotonic()+20
        ready = ''
        while time.monotonic()<deadline:
            command(ser, '')
            ready = command(ser, 'REST stats')
            if 'CLOCK ' in ready: break
        assert 'CLOCK ' in ready, 'Panel console did not become ready'
        command(ser, 'AGENT\tv1\t1')
        command(ser, 'TODOIST\tv1\t1\t0\t0')  # dismiss startup loader, RAM only
        log += command(ser, 'REST saver')
        log += command(ser, 'PERF')
        log += command(ser, 'REST stats')
        first = re.findall(r'CLOCK .*digits=(\d+)', log)
        assert first, 'Panel did not confirm screensaver statistics'
        assert len(first[-1]) == 6, 'Enable seconds in Clock before this test'
        key = 'clock-load'
        command(ser, f'MEDIA\tv1\t1\tplaying\tClock load test\tExample artist\tMusic\tExample album\t2026\t{key}')
        data = b'\x00\xf8'*(200*200)
        total = (len(data)+479)//480
        start = time.monotonic()
        last_stats = start
        index = 0
        while time.monotonic()-start < 65:
            # Large, valid art packets keep the console busy while the clock runs.
            chunk = data[index*480:(index+1)*480].hex()
            ser.write(f'MEDIA_COVER\tv1\t1\t{key}\t200\t200\trgb565le\t{total}\t{index}\t{chunk}\n'.encode())
            ser.flush()
            index = (index+1) % total
            ser.timeout = .004
            log += ser.read(8192).decode('utf8','replace')
            if time.monotonic()-last_stats >= 1:
                ser.write(b'REST stats\nAGENT\tv1\t1\n')
                last_stats = time.monotonic()
            time.sleep(.046)
        ser.timeout = .2
        log += command(ser, 'REST stats')
        log += command(ser, 'PERF')
        (OUT/'timing.log').write_text('\n'.join(line for line in log.splitlines() if line.startswith(('CLOCK ', 'PERF ', 'rest ', 'MEDIA_COVER_OK'))))
        rows = re.findall(r'CLOCK gap_max=(\d+) changes=(\d+) snaps=(\d+) active=(\d+) digits=(\d+)',log)
        assert rows, 'Missing timing telemetry'
        gap, changes, snaps, _, _ = rows[-1]
        result = {'max_ui_gap_ms':int(gap),'wall_time_updates':int(changes),'resyncs':int(snaps),'samples':len(rows),'duration_s':round(time.monotonic()-start,1),'artwork_acks':log.count('MEDIA_COVER_OK')}
        (OUT/'result.json').write_text(json.dumps(result,indent=2))
        snapshot(ser,str(OUT/'screensaver.png'))
        assert int(gap)<250, result
        assert int(changes)>=62, result
        assert result['artwork_acks']>0, result
        print('PASS',json.dumps(result))
    finally:
        if ser:
            try: command(ser,'REST wake')
            finally: ser.close()
        if running:
            subprocess.run(['launchctl','bootstrap',f'gui/{os.getuid()}',plist],capture_output=True)

if __name__ == '__main__': main()
