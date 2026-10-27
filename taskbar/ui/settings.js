import { invoke, emit, listen, getConfig, setConfig } from "./common.js";

const $ = (id) => document.getElementById(id);
const status = $("status");

// id -> where it lives in the config. Keeps read-back and write-out from
// drifting apart as the list grows.
const FIELDS = {
  edge: ["edge"],
  thickness: ["thickness"],
  replaceShell: ["replaceShell"],
  background: ["theme", "background"],
  foreground: ["theme", "foreground"],
  muted: ["theme", "muted"],
  accent: ["theme", "accent"],
  opacity: ["theme", "opacity"],
  blur: ["theme", "blur"],
  radius: ["theme", "radius"],
  margin: ["theme", "margin"],
  gap: ["theme", "gap"],
  fontSize: ["theme", "fontSize"],
  fontFamily: ["theme", "fontFamily"],
  clock: ["widgets", "clock"],
  clockSeconds: ["widgets", "clockSeconds"],
  mic: ["widgets", "mic"],
  apps: ["widgets", "apps"],
  appsMax: ["widgets", "appsMax"],
  wifi: ["widgets", "wifi"],
  search: ["widgets", "search"],
  taskView: ["widgets", "taskView"],
  volume: ["widgets", "volume"],
  battery: ["widgets", "battery"],
  showDesktop: ["widgets", "showDesktop"],
  language: ["widgets", "language"],
  mixer: ["widgets", "mixer"],
  tray: ["widgets", "tray"],
  vpn: ["widgets", "vpn"],
  media: ["widgets", "media"],
  claude: ["widgets", "claude"],
  claudeCommand: ["widgets", "claudeCommand"],
};

// Opacity is a 0..1 float in the config but a 15..100 slider here.
const SCALED = new Set(["opacity"]);

const at = (obj, path) => path.reduce((o, k) => o[k], obj);
const put = (obj, path, v) => {
  const last = path[path.length - 1];
  path.slice(0, -1).reduce((o, k) => o[k], obj)[last] = v;
};

let cfg = await getConfig();
let writing = false;

function paint() {
  for (const [id, path] of Object.entries(FIELDS)) {
    const el = $(id);
    if (!el) continue;
    let v = at(cfg, path);
    if (SCALED.has(id)) v = Math.round(v * 100);
    if (el.type === "checkbox") el.checked = !!v;
    else el.value = v;
    const out = $(id + "Val");
    if (out) out.textContent = SCALED.has(id) ? v + "%" : v;
  }
}

function collect() {
  const next = structuredClone(cfg);
  for (const [id, path] of Object.entries(FIELDS)) {
    const el = $(id);
    if (!el || el.disabled) continue;
    let v;
    if (el.type === "checkbox") v = el.checked;
    else if (el.type === "range") v = SCALED.has(id) ? Number(el.value) / 100 : Number(el.value);
    else v = el.value;
    put(next, path, v);
  }
  return next;
}

let pending = null;
async function push() {
  // Dragging a slider fires continuously; collapse those into one write so the
  // config file is not rewritten sixty times a second.
  clearTimeout(pending);
  pending = setTimeout(async () => {
    writing = true;
    try {
      cfg = await setConfig(collect());
      status.textContent = "salvo";
    } catch (e) {
      status.textContent = String(e);
    } finally {
      writing = false;
    }
    paint();
    setTimeout(() => (status.textContent = ""), 1200);
  }, 120);
}

for (const id of Object.keys(FIELDS)) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("input", () => {
    const out = $(id + "Val");
    if (out) out.textContent = SCALED.has(id) ? el.value + "%" : el.value;
    push();
  });
}

$("reset").onclick = async () => {
  cfg = await invoke("reset_config");
  await emit("config-changed", cfg);
  paint();
};

listen("config-changed", (e) => {
  if (writing) return;
  cfg = e.payload;
  paint();
});

paint();
