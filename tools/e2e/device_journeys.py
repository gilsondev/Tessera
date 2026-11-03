"""Repeatable touch and USB journeys on the real JC3248.

Temporarily pauses only com.tessera.agentd, restores it in finally, and injects
synthetic RAM snapshots. Does not change Wi-Fi, accounts, tokens or preferences.
Requires portrait 320x480 and the Music widget enabled. Outputs stay in /tmp.
"""
import argparse
import glob
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import serial
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from panel_probe import snapshot
OUT=Path('/tmp/tessera-device-e2e')

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--port'); args=parser.parse_args()
    OUT.mkdir(exist_ok=True)
    plist=Path.home()/'Library/LaunchAgents/com.tessera.agentd.plist'
    service=f'gui/{os.getuid()}/com.tessera.agentd'
    running=subprocess.run(['launchctl','print',service],capture_output=True).returncode==0
    ser=None
    try:
        if running:
            subprocess.run(['launchctl','bootout',f'gui/{os.getuid()}',str(plist)],check=True,capture_output=True)
            time.sleep(.5)
        ports=[args.port] if args.port else sorted(p for p in glob.glob('/dev/cu.usbmodem*') if not p.endswith('1401'))
        if len(ports)!=1: raise RuntimeError('Specify --port when zero or multiple panels are present')
        ser=serial.Serial();ser.port=ports[0];ser.baudrate=115200;ser.timeout=.1;ser.dtr=True;ser.rts=False;ser.open();time.sleep(.7);ser.reset_input_buffer()
        def read(seconds=.5):
            end=time.monotonic()+seconds; data=bytearray()
            while time.monotonic()<end: data.extend(ser.read(4096))
            return data.decode('utf8','replace')
        last_heartbeat=0.0
        def send(line,seconds=.3):
            nonlocal last_heartbeat
            if time.monotonic()-last_heartbeat>5:
                ser.write(b'AGENT\tv1\t1\n');ser.flush();time.sleep(.01);last_heartbeat=time.monotonic()
            ser.write((line+'\n').encode());ser.flush();return read(seconds)
        def tree():
            raw=send('UI_TREE',.3)
            deadline=time.monotonic()+6
            while 'UI_TREE_END' not in raw and time.monotonic()<deadline: raw+=read(.2)
            if 'UI_TREE_END' not in raw:
                (OUT/'failed-tree.log').write_text(raw)
                raise AssertionError('UI_TREE timed out; check USB ownership and firmware version')
            header=next(l for l in raw.splitlines() if l.startswith('UI_TREE_BEGIN'))
            assert header.split()[1:]==['320','480'],'Device journey currently requires portrait 320x480'
            return [json.loads(l.split('\t',1)[1]) for l in raw.splitlines() if l.startswith('UI_NODE\t')]
        def expect(text):
            rows=tree();assert any(text in r['text'] for r in rows),f'Missing UI text: {text}'
            return rows
        def home():
            assert any(r['text']=='\uf013' and r['y']<60 for r in tree()),'Expected home gear button'
        def tap(x,y): send(f'DRAG {x} {y} {x} {y} 120',1.0)
        def click(text):
            for _ in range(6):
                rows=tree(); row=next((r for r in rows if text in r['text'] and r['clickable'] and 40<r['y']<440),None)
                if row is not None: tap(row['x'],row['y']); return
                send('DRAG 160 400 160 180 400',.7)
            raise AssertionError(f'No visible touch target: {text}')
        send('REST wake')
        # Recover home by user-like back taps. No synthetic navigation shortcut.
        for _ in range(6):
            if any(r['text']=='\uf013' and r['y']<60 for r in tree()): break
            tap(32,28)
        home()
        click('Clima')
        weather_rows=tree()
        assert any(r['text']=='\uf053' and r['clickable'] and r['y']<60 for r in weather_rows), 'Expected weather detail back button'
        assert not any(r['text']=='\uf013' and r['y']<60 for r in weather_rows), 'Weather tap stayed on home'
        # Forecast availability is external; navigation also works in loading/error states.
        snapshot(ser,str(OUT/'weather.png'));tap(32,28);home()
        send('AGENT\tv1\t1')
        send('MEDIA\tv1\t1\tplaying\tE2E first song\tExample artist\tMusic\tExample album\t2026\te2e-first')
        for _ in range(6):
            if any('E2E first song' in r['text'] and 40<r['y']<440 for r in tree()): break
            send('DRAG 160 400 160 160 400',.8)
        click('E2E first song');expect('Example artist');expect('Example album · 2026')
        def cover(key, omit=None):
            data=b'\x00\xf8'*(200*200); count=(len(data)+479)//480; output=''
            for i in range(count):
                if i==omit: continue
                line=f'MEDIA_COVER\tv1\t1\t{key}\t200\t200\trgb565le\t{count}\t{i}\t{data[i*480:(i+1)*480].hex()}\n'
                ser.write(line.encode());ser.flush();time.sleep(.002)
                if ser.in_waiting: output+=ser.read(ser.in_waiting).decode('utf8','replace')
            return output+read(.8)
        # Missing chunk never completes; a full retry must recover and ACK.
        assert 'MEDIA_COVER_OK\te2e-first' not in cover('e2e-first',omit=3)
        assert 'MEDIA_COVER_OK\te2e-first' in cover('e2e-first')
        snapshot(ser,str(OUT/'music-cover.png'))
        send('MEDIA\tv1\t1\tplaying\tE2E next song\tNext artist\tMusic\tNext album\t2025\te2e-next')
        expect('E2E next song');expect('Next album · 2025')
        assert 'MEDIA_COVER_OK' not in cover('e2e-first'), 'Previous-track cover must be rejected'
        assert 'MEDIA_COVER_OK\te2e-next' in cover('e2e-next')
        send('MEDIA\tv1\t1\tpaused\tE2E next song\tNext artist\tMusic\tNext album\t2025\te2e-next')
        expect('pausado')
        send('MEDIA\tv1\t1\tstopped||||||'.replace('|','\t'));expect('Nada tocando')
        tap(32,28);home()
        perf=send('PERF');(OUT/'perf.log').write_text(perf)
        result={'passed':True,'checks':['touch weather and back','music metadata and album/year','missing cover chunk rejected','cover retry acknowledged','old track cover rejected','next track cover acknowledged','paused/stopped states'], 'synthetic_data':True}
        (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));print('Artifacts:',OUT)
    finally:
        if ser: ser.close()
        if running:
            subprocess.run(['launchctl','bootstrap',f'gui/{os.getuid()}',str(plist)],capture_output=True)
            subprocess.run(['launchctl','kickstart','-k',service],capture_output=True)
if __name__=='__main__': main()
