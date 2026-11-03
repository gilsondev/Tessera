"""Run with: browser-harness < tools/e2e/browser_journeys.py

Requires tools/e2e/serve.py. Uses real pointer clicks with accessibility names,
production JS, isolated HTTP fixtures and explicit assertions. No live accounts.
"""
import base64
import json
import pathlib
import time
import urllib.request
OUT = pathlib.Path('/tmp/tessera-browser-e2e')
OUT.mkdir(exist_ok=True)
def state(**patch):
    req=urllib.request.Request('http://127.0.0.1:19793/__test/state',json.dumps(patch).encode(),{'Content-Type':'application/json'})
    return json.load(urllib.request.urlopen(req))
def reset():
    urllib.request.urlopen(urllib.request.Request('http://127.0.0.1:19793/__test/reset',b'{}',{'Content-Type':'application/json'})).read()
def evaluate(expression):
    result=cdp('Runtime.evaluate',expression=expression,returnByValue=True)
    assert 'exceptionDetails' not in result, result
    return result['result'].get('value')
def until(expression, timeout=8):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        if evaluate(expression): return
        time.sleep(.1)
    raise AssertionError('Timed out: '+expression)
def click_name(name, role='button'):
    nodes=cdp('Accessibility.getFullAXTree')['nodes']
    matches=[n for n in nodes if n.get('role',{}).get('value')==role and n.get('name',{}).get('value')==name and not n.get('ignored')]
    assert matches, f'No accessible {role}: {name}; available: '+str([(n.get('role',{}).get('value'),n.get('name',{}).get('value')) for n in nodes if n.get('role',{}).get('value') in ['button','checkbox']])
    node=matches[0]['backendDOMNodeId']
    cdp('DOM.scrollIntoViewIfNeeded',backendNodeId=node)
    quad=cdp('DOM.getBoxModel',backendNodeId=node)['model']['content']
    click_at_xy(sum(quad[0::2])/4,sum(quad[1::2])/4)
def screen(name): until(f'document.querySelector(".screen.on")?.dataset.screen === {json.dumps(name)}')
def shot(name):
    raw=cdp('Page.captureScreenshot',format='png')['data']
    (OUT/name).write_bytes(base64.b64decode(raw))
def viewport(width,height):
    cdp('Emulation.setDeviceMetricsOverride',width=width,height=height,deviceScaleFactor=1,mobile=False)
    cdp('Emulation.setEmulatedMedia',features=[{'name':'prefers-reduced-motion','value':'reduce'}])

reset()
new_tab('http://127.0.0.1:1420/agent/')
viewport(980,720)
wait_for_load(); screen('welcome')
assert evaluate('document.getElementById("tabs").hidden')
shot('agent-welcome.png')
click_name('Começar neste Mac'); screen('scan-result')
assert evaluate('[...document.querySelectorAll(".found .need")].every(b=>b.getBoundingClientRect().width < 180)')
shot('agent-discovery-controls.png')
click_name('Resolver autorizações'); screen('setup')
click_name('Continuar'); screen('setup-cable')
until('document.querySelector("[data-action=finish-setup]").disabled')
assert not state()['setup_completed']
state(usb='up')
until('!document.querySelector("[data-action=finish-setup]").disabled')
click_name('Concluir configuração'); screen('home')
assert state()['setup_completed']
cdp('Page.reload',ignoreCache=True); wait_for_load(); screen('home')
click_name('Integrações'); screen('mac')
until('document.querySelector("[data-cap-toggle=media]").classList.contains("on")')
assert evaluate('document.querySelector("[data-cap-toggle=media]").getBoundingClientRect().height === 26')
state(toggle_delay_ms=1500)
unchanged_panel_calls=state()['panel_settings_calls']
click_name('Coletar música neste Mac')
until('!document.querySelector("[data-cap-toggle=media]").classList.contains("on")',timeout=.4)
click_name('Coletar música neste Mac')
until('document.querySelector("[data-cap-toggle=media]").classList.contains("on")',timeout=.4)
until('!document.querySelector("[data-cap-toggle=media]").hasAttribute("aria-busy")',timeout=8)
assert state()['caps']['media'] is True
assert state()['panel_settings_calls']==unchanged_panel_calls
state(toggle_delay_ms=0)
shot('agent-controls.png')
click_name('Configurar Agenda'); screen('calendar')
until('!document.querySelector("[data-action=calendar-connect]").disabled')
click_name('Permitir Calendários')
until('document.querySelectorAll("[data-calendar-choice]").length === 2')
click_name('Equipe de exemplo · iCloud',role='checkbox')
until('document.querySelector("[data-calendar-choice=work]").checked')
assert state()['selected']==['work']
shot('agent-calendars.png')
state(permission='denied')
until('document.querySelector("[data-live=calendar-status]").textContent.includes("Ajustes")')
assert evaluate('document.querySelectorAll("[data-calendar-choice]").length === 0')
state(permission='authorized')
click_name('Voltar'); screen('mac')
click_name('Painel'); screen('panel')
until('!document.querySelector("[data-live=panel-controls]").hidden')
click_name('Alto')
until('document.querySelector("[data-live=panel-brightness-sub]").textContent.includes("100")')
assert state()['settings']['brightness']==100
click_name('Ajuda e app'); screen('more')
click_name('Rodar'); screen('doctor')
until('document.querySelector("[data-live=doctor-checks]").textContent.includes("Agente disponível")')
assert evaluate('document.querySelector("[data-live=doctor-demo-badge]").hidden')
state(reachable=False)
click_name('Widgets'); screen('widgets')
until('!document.querySelector("[data-live=widgets-offline-banner]").hidden')
shot('agent-panel-offline.png')
state(error=True)
click_name('Início'); screen('home')
until('document.querySelector("[data-live=home-status-title]").textContent.includes("Agente")')
shot('agent-daemon-offline.png')
state(error=False,reachable=True)
viewport(390,844)
click_name('Integrações'); screen('mac')
assert evaluate('document.documentElement.scrollWidth <= window.innerWidth')
shot('agent-narrow.png')
print('PASS Agent: compact controls, immediate switches during delayed saves, last-click-wins, independent collection/display, first-use, real USB gate, persisted setup, native calendar grant/selection/denial, network offline, daemon error, narrow layout')

new_tab('http://127.0.0.1:1420/Tessera/')
viewport(1280,900); wait_for_load()
until('document.getElementById("version-note").textContent.includes("0.1.0")')
assert evaluate('document.querySelectorAll(".setup-list > li").length === 4')
assert evaluate('[...document.images].every(i=>i.complete && i.naturalWidth>0)')
assert evaluate('document.getElementById("download").getAttribute("href") === "/Tessera/install.command"')
assert evaluate('![...document.querySelectorAll("a")].some(a=>a.href.includes("Tessara"))')
shot('site-desktop.png')
viewport(390,844)
assert evaluate('document.documentElement.scrollWidth <= window.innerWidth')
shot('site-mobile.png')
click_name('Preparar meu Tessera',role='link')
until('document.getElementById("comecar").getBoundingClientRect().top < 100')
shot('site-start-mobile.png')
print('PASS Site: introduction, four-step setup, real images, manifest version, complete installer link, canonical name, mobile navigation')
print('Artifacts:',OUT)
