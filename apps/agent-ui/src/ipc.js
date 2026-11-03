

/** agentd localhost IPC (TESSERA_AGENTD_IPC_PORT / VITE_AGENTD_PORT, default 19792). */
export const AGENTD_PORT = Number(
  (typeof import.meta !== "undefined" &&
    import.meta.env &&
    import.meta.env.VITE_AGENTD_PORT) ||
    19792,
);

export function agentdUrl(path) {
  return `http://127.0.0.1:${AGENTD_PORT}${path}`;
}

const IPC_TIMEOUT_MS = 1500;

export async function fetchStatus() {
  const r = await fetch(agentdUrl("/v1/status"), {
    signal: AbortSignal.timeout(800),
  });
  if (!r.ok) throw new Error(`status ${r.status}`);
  const body = await r.json();
  if (body?.ok !== true) throw new Error("agentd not ok");
  return body;
}

/** @returns {Promise<{ scanned_at: string, found: object[] }>} */
export async function fetchScanLast() {
  const r = await fetch(agentdUrl("/v1/scan/last"), {
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS),
  });
  if (!r.ok) throw new Error(`scan/last ${r.status}`);
  return r.json();
}

/**
 * @param {object} [body]
 * @returns {Promise<{ scanned_at: string, found: object[] }>}
 */
export async function postScan(body = {}) {
  const r = await fetch(agentdUrl("/v1/scan"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(10000),
  });
  if (!r.ok) throw new Error(`scan ${r.status}`);
  return r.json();
}

/** @returns {Promise<{ machines: object[] }>} */
export async function fetchMachines() {
  const r = await fetch(agentdUrl("/v1/machines"), {
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS),
  });
  if (!r.ok) throw new Error(`machines ${r.status}`);
  return r.json();
}

/**
 * @returns {Promise<{ report: { summary: string, checks: object[] }, demo: boolean }>}
 */
let doctorPending = null;
export async function fetchDoctor() {
  if (!doctorPending) doctorPending = readDoctor().finally(() => { doctorPending = null; });
  return doctorPending;
}
async function readDoctor() {
  try {
    const r = await fetch(agentdUrl("/v1/doctor"), {
      signal: AbortSignal.timeout(10000),
    });
    if (r.status === 404 || r.status === 501) {
      return { report: { summary: "unavailable", checks: [{ id: "agentd", level: "bad", title: "Diagnóstico indisponível", detail: "Abra o Tessera em segundo plano e tente novamente." }] }, demo: false };
    }
    if (!r.ok) throw new Error(`doctor ${r.status}`);
    const report = await r.json();
    if (!report?.checks?.length) throw new Error("doctor empty");
    return { report, demo: false };
  } catch {
    return { report: { summary: "unavailable", checks: [{ id: "agentd", level: "bad", title: "Diagnóstico indisponível", detail: "Abra o Tessera em segundo plano e tente novamente." }] }, demo: false };
  }
}

/**
 * @param {number} [tail=200]
 * @returns {Promise<{ lines: string[], demo: boolean }>}
 */
export async function fetchLogs(tail = 200) {
  const q = Number.isFinite(tail) ? Math.max(1, Math.min(tail, 2000)) : 200;
  try {
    const r = await fetch(agentdUrl(`/v1/logs?tail=${q}`), {
      signal: AbortSignal.timeout(IPC_TIMEOUT_MS),
    });
    if (r.status === 404 || r.status === 501) {
      return { lines: ["Não foi possível carregar o histórico. Verifique a conexão com o Agente."], demo: false };
    }
    if (!r.ok) throw new Error(`logs ${r.status}`);
    const body = await r.json();
    const lines = Array.isArray(body?.lines) ? body.lines : [];

    return { lines, demo: false };
  } catch {
    return { lines: ["Não foi possível carregar o histórico. Verifique a conexão com o Agente."], demo: false };
  }
}

/** @returns {Promise<object>} */
export async function fetchPanelState() {
  const r = await fetch(agentdUrl("/v1/panel/state"), {
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 3),
  });
  if (!r.ok) throw new Error(`panel/state ${r.status}`);
  return r.json();
}

/** @param {object} patch */
export async function postPanelSettings(patch) {
  const r = await fetch(agentdUrl("/v1/panel/settings"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(patch),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 3),
  });
  if (!r.ok) throw new Error(`panel/settings ${r.status}`);
  return r.json();
}

/**
 * Namespaced widget setting on the device (portal `/api/widget_settings`).
 * @param {string} id widget id e.g. org.tessera.calendar
 * @param {string} key e.g. home_tile_size
 * @param {string|number|boolean} value
 */
export async function postWidgetSettings(id, key, value) {
  const r = await fetch(agentdUrl("/v1/panel/widget_settings"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id, key, value }),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 3),
  });
  if (!r.ok) throw new Error(`panel/widget_settings ${r.status}`);
  return r.json();
}

/**
 * Show/hide widgets on the panel home (portal `/api/dashboard`).
 * @param {{ id: string, title?: string, shown: boolean }[]} widgets
 */
export async function postPanelDashboard(widgets) {
  const r = await fetch(agentdUrl("/v1/panel/dashboard"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ widgets }),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 3),
  });
  if (!r.ok) throw new Error(`panel/dashboard ${r.status}`);
  return r.json();
}

export async function postPanelRefresh() {
  const r = await fetch(agentdUrl("/v1/panel/refresh"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: "{}",
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 3),
  });
  if (!r.ok) throw new Error(`panel/refresh ${r.status}`);
  return r.json();
}

/** @param {string} token */
export async function postPanelToken(token) {
  const r = await fetch(agentdUrl("/v1/panel/token"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token }),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 4),
  });
  if (!r.ok) throw new Error(`panel/token ${r.status}`);
  return r.json();
}

/** @param {object} body city | {name,latitude,longitude} */
export async function postPanelWeather(body) {
  const r = await fetch(agentdUrl("/v1/panel/weather"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 4),
  });
  if (!r.ok) throw new Error(`panel/weather ${r.status}`);
  return r.json();
}

/** @param {string} selection comma-separated calendar ids */
export async function postCalendarSelection(selection) {
  const r = await fetch(agentdUrl("/v1/calendar/selection"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(selection.split(",").filter(Boolean)),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 4),
  });
  if (!r.ok) throw new Error(`panel/calendar/selection ${r.status}`);
  return r.json();
}

export async function postCalendarAddAccount() {
  const r = await fetch(agentdUrl("/v1/calendar/connect"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: "{}",
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 4),
  });
  if (!r.ok) throw new Error(`panel/calendar/add_account ${r.status}`);
  return r.json();
}

/** @returns {Promise<object>} */
export async function fetchUpdate() {
  const r = await fetch(agentdUrl("/v1/update"), {
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 2),
  });
  if (!r.ok) throw new Error(`update ${r.status}`);
  return r.json();
}

/**
 * @param {{ auto_update?: boolean, check?: boolean }} body
 * @returns {Promise<object>}
 */
export async function postUpdate(body = {}) {
  const r = await fetch(agentdUrl("/v1/update"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 30),
  });
  if (!r.ok) throw new Error(`update ${r.status}`);
  return r.json();
}

/**
 * Register Tessera in the TCC list for `pane`, return deep-link + hint.
 * @param {string} pane full_disk_access | automation | login_items | calendar
 * @returns {Promise<{ ok: boolean, pane: string, settings_url: string, binary_path: string, hint: string }>}
 */
export async function preparePermission(pane) {
  const r = await fetch(agentdUrl("/v1/permissions/prepare"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ pane }),
    signal: AbortSignal.timeout(IPC_TIMEOUT_MS * 2),
  });
  if (!r.ok) throw new Error(`permissions/prepare ${r.status}`);
  return r.json();
}

/**
 * Open System Settings to the privacy pane (Tauri) or fallback `window.open`.
 * @param {string} pane
 */
export async function openPrivacySettings(pane) {
  const prep = await preparePermission(pane);
  // Reveal Tessera.app (brand icon) in Finder for FDA — never a Terminal binary.
  const reveal =
    pane === "full_disk_access" || pane === "automation"
      ? prep.binary_path || null
      : null;
  const invoke = window.__TAURI__?.core?.invoke;
  if (typeof invoke === "function") {
    await invoke("open_privacy_settings", {
      url: prep.settings_url,
      revealPath: reveal,
    });
  } else {
    window.open(prep.settings_url, "_blank");
  }
  return prep;
}

/** agentd reachable and healthy (`ok: true` in /v1/status). */
export function isAgentdOnline(status) {
  return status?.ok === true;
}

export function isUsbUp(status) {
  return status?.usb === "up";
}

export async function calendarRequest(path = "", body) {
  const r = await fetch(agentdUrl(`/v1/calendar${path}`), {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json" },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    signal: AbortSignal.timeout(5000),
  });
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}
