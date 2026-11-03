"""Isolated browser test backend: synthetic data, never the installed Agent.

Serves the production UI at :1420/agent/ and a mock IPC at :19793. Only the
served ipc.js port is rewritten; no test switch ships in the product.
"""
import json
import mimetypes
import pathlib
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit
ROOT = pathlib.Path(__file__).resolve().parents[2]
LOCK = threading.RLock()
STATE = {}
def reset():
    STATE.clear()
    STATE.update(setup_completed=False, usb='disconnected', reachable=True, permission='not_determined', selected=[], error=False,
                 toggle_delay_ms=0,caps={'media':True,'focus':True,'calendar':True,'codex':True},panel_settings_calls=0,
                 settings={'brightness':60,'portrait':True,'appearance':0,'sound':True,'rest_minutes':5,'clock_minutes':2})
reset()
def calendars():
    return [{'id':k,'name':n,'account':a,'color':'1A73E8','selected':k in STATE['selected']} for k,n,a in [('work','Equipe de exemplo','iCloud'),('personal','Pessoal de exemplo','No meu Mac')]]
def agenda():
    return {'configured':True,'pending':False,'linked':STATE['permission']=='authorized','permission':STATE['permission'],
            'message': {'authorized':'Escolha os calendários que deseja mostrar.','denied':'Permita Calendários para Tessera Calendar nos Ajustes do Sistema.'}.get(STATE['permission'],'Permita a leitura dos calendários já adicionados ao Mac.'),
            'calendars':calendars() if STATE['permission']=='authorized' else []}
def scan():
    return {'scanned_at':'2026-10-28T12:00:00Z','found':[{'id':k,'state':'needs_auth' if k=='calendar' and STATE['permission']!='authorized' else 'ready','enabled':STATE['caps'].get(k,True)} for k in ['media','focus','calendar','codex']]}
def panel():
    return {'ok':STATE['reachable'],'reachable':STATE['reachable'],'settings':STATE['settings'],'device_settings':STATE['settings'],
            'info':{'host':'tessera.local'},'widgets':[{'id':'org.tessera.'+k,'title':n,'shown':True} for k,n in [('clock','Relógio'),('calendar','Agenda'),('media','Música'),('mac','Foco')]],
            'widget_settings':{'org.tessera.calendar':{'home_tile_size':'M'}},'calendar':{'calendars':calendars()},'weather':{},'token_set':False}
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def respond(self, body, status=200, content='application/json'):
        raw = body if isinstance(body,bytes) else json.dumps(body).encode()
        self.send_response(status); self.send_header('Content-Type',content)
        self.send_header('Access-Control-Allow-Origin','http://127.0.0.1:1420')
        self.send_header('Access-Control-Allow-Methods','GET, POST, OPTIONS');self.send_header('Access-Control-Allow-Headers','Content-Type')
        self.send_header('Cache-Control','no-store'); self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
    def do_OPTIONS(self): self.respond({})
    def do_POST(self):
        data = json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))) or b'{}')
        if urlsplit(self.path).path=='/v1/scan' and isinstance(data,dict) and data.get('found'): time.sleep(STATE.get('toggle_delay_ms',0)/1000)
        with LOCK:
            path=urlsplit(self.path).path
            if path=='/__test/reset': reset(); return self.respond(STATE)
            if path=='/__test/state': STATE.update(data); return self.respond(STATE)
            if STATE['error']: return self.respond({'error':'Synthetic failure'},503)
            if path=='/v1/scan':
                if 'setup_completed' in data: STATE['setup_completed']=data['setup_completed']
                for item in data.get('found',[]): STATE['caps'][item['id']]=item['enabled']
                return self.respond(scan())
            if path=='/v1/calendar/connect': STATE['permission']='authorized'; return self.respond(agenda())
            if path=='/v1/calendar/selection': STATE['selected']=data; return self.respond(agenda())
            if path=='/v1/panel/settings': STATE['panel_settings_calls']+=1; STATE['settings'].update(data); return self.respond(panel())
            if path.startswith('/v1/panel/'): return self.respond(panel())
            return self.respond({})
    def do_GET(self):
        with LOCK:
            path=urlsplit(self.path).path
            if path=='/__test/state': return self.respond(STATE)
            if path.startswith('/v1/'):
                if STATE['error']: return self.respond({'error':'Synthetic failure'},503)
                if path=='/v1/status': return self.respond({'ok':True,'version':'test','usb':STATE['usb'],'setup_completed':STATE['setup_completed'],'machine':{'id':'synthetic','display_name':'Mac de exemplo'}})
                if path=='/v1/scan/last': return self.respond(scan())
                if path=='/v1/calendar': return self.respond(agenda())
                if path=='/v1/panel/state': return self.respond(panel())
                if path=='/v1/doctor': return self.respond({'summary':'ok','checks':[{'id':'usb','level':'ok','title':'Agente disponível','detail':'Dados de teste sintéticos'}]})
                if path=='/v1/update': return self.respond({'current_version':'test','auto_update':False,'update_available':False})
                if path=='/v1/logs': return self.respond({'lines':[]})
                if path=='/v1/machines': return self.respond({'machines':[{'machine_id':'synthetic','display_name':'Mac de exemplo','this_machine':True}]})
                return self.respond({},404)
        if path.startswith('/Tessera'):
            rel=path.removeprefix('/Tessera').lstrip('/') or 'index.html'; base=ROOT/'site/dist'
        else:
            rel=path.removeprefix('/agent').lstrip('/') or 'index.html'; base=ROOT/'apps/agent-ui/src'
        file=(base/rel).resolve()
        if not file.is_relative_to(base.resolve()) or not file.is_file(): return self.respond({},404)
        raw=file.read_bytes()
        if file.name=='ipc.js': raw=raw.replace(b'19792',b'19793')
        self.respond(raw,content=mimetypes.guess_type(str(file))[0] or 'application/octet-stream')
if __name__=='__main__':
    for port in (1420,19793):
        server=ThreadingHTTPServer(('127.0.0.1',port),Handler)
        threading.Thread(target=server.serve_forever,daemon=True).start()
    print('Synthetic UI and site preview: http://127.0.0.1:1420/agent/ and /Tessera/',flush=True)
    threading.Event().wait()
