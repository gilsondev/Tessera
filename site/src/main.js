const LATEST = new URL(`${import.meta.env.BASE_URL}latest.json`, window.location.origin).href;

async function loadLatest() {
  const note = document.getElementById("version-note");
  const dl = document.getElementById("download");
  const foot = document.getElementById("footer-ver");
  try {
    const r = await fetch(LATEST, { cache: "no-store" });
    if (!r.ok) throw new Error(String(r.status));
    const j = await r.json();
    const ver = j.version || "?";
    if (note) {
      note.textContent = j.released_at
        ? `Versão ${ver} · ${String(j.released_at).slice(0, 10)}`
        : `Versão ${ver}`;
    }
    if (foot) foot.textContent = `Agent ${ver}`;
    const href = `${import.meta.env.BASE_URL}install.command`;
    if (dl && href) {
      dl.href = href;
      dl.textContent = `Instalar Agente ${ver}`;
    }
  } catch {
    if (note) note.textContent = "Release mais recente no GitHub.";
    if (foot) foot.textContent = "";
  }
}

function wireCopy() {
  const btn = document.getElementById("copy-cmd");
  const cmd = document.getElementById("install-cmd");
  if (!btn || !cmd) return;
  btn.addEventListener("click", async () => {
    const text = cmd.textContent || "";
    try {
      await navigator.clipboard.writeText(text);
      btn.textContent = "Copiado";
      setTimeout(() => {
        btn.textContent = "Copiar";
      }, 1600);
    } catch {
      btn.textContent = "Selecione e ⌘C";
    }
  });
}

void loadLatest();
wireCopy();
