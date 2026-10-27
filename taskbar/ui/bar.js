import { invoke, listen, getConfig, applyTheme, icon } from "./common.js";

const left = document.getElementById("left");
const right = document.getElementById("right");
let cfg = await getConfig();

function pill(zone, id, title) {
  const b = document.createElement("button");
  b.className = "pill";
  b.id = id;
  b.title = title;
  zone.appendChild(b);
  return b;
}

// ---- microphone ------------------------------------------------------------
// The only state the bar owns that something else can change underneath it, so
// it is re-read on a timer as well as after our own toggle.
let micBtn = null;

function paintMic(muted) {
  if (!micBtn) return;
  micBtn.classList.toggle("hidden", muted === null);
  if (muted === null) return;
  micBtn.replaceChildren(icon(muted ? "micOff" : "mic"));
  micBtn.classList.toggle("alert", muted);
  micBtn.title = muted ? "Microfone mudo — clique para ativar" : "Microfone ativo — clique para mutar";
}

async function refreshMic() {
  paintMic(await invoke("mic_muted"));
}

// ---- clock -----------------------------------------------------------------
let clockEl = null;

function paintClock() {
  if (!clockEl) return;
  const now = new Date();
  const time = now.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    ...(cfg.widgets.clockSeconds ? { second: "2-digit" } : {}),
  });
  const date = now.toLocaleDateString(undefined, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
  clockEl.replaceChildren(
    Object.assign(document.createElement("span"), { textContent: time }),
    Object.assign(document.createElement("span"), { className: "date", textContent: date }),
  );
  clockEl.title = now.toLocaleString();
}

// ---- layout ----------------------------------------------------------------
// Rebuilt whole rather than diffed: a dozen nodes, and it keeps the widget
// order honest against the config.
const center = document.getElementById("center");

function build() {
  left.replaceChildren();
  center.replaceChildren();
  right.replaceChildren();
  micBtn = null;
  clockEl = null;

  // ---- center: Start + apps ------------------------------------------------
  const start = pill(center, "start", "Menu Iniciar");
  start.classList.add("square");
  start.appendChild(icon("start"));
  start.onclick = () => invoke("open_start_menu").catch(() => {});
  // The apps list (populated by a later refresh) sits right after Start,
  // in the same centre strip, so Start/apps read as one unit.
  const appsHost = document.createElement("div");
  appsHost.id = "apps";
  appsHost.className = "zone";
  center.appendChild(appsHost);
  paintApps();

  // ---- right: status widgets ----------------------------------------------
  if (cfg.widgets.claude) {
    const b = pill(right, "claude", "Abrir o Claude Code");
    b.classList.add("square");
    b.appendChild(icon("terminal"));
    b.onclick = () => invoke("launch_claude").catch(() => {});
  }

  if (cfg.widgets.mic) {
    micBtn = pill(right, "mic", "Microfone");
    micBtn.classList.add("square");
    micBtn.onclick = async () => paintMic(await invoke("toggle_mic"));
    refreshMic();
  }

  const gear = pill(right, "settings", "Ajustes do codabar");
  gear.classList.add("square");
  gear.appendChild(icon("settings"));
  gear.onclick = () => invoke("open_settings").catch(() => {});

  if (cfg.widgets.clock) {
    clockEl = pill(right, "clock", "");
    paintClock();
  }
}

// ---- open windows ----------------------------------------------------------
// The backend pushes the list once a second, but only when it differs. Nodes
// are reused by hwnd so an icon never blinks: a refresh that only moved the
// focus touches two class lists and nothing else.
let apps = [];

function paintApps() {
  const host = document.getElementById("apps");
  if (!host) return;
  if (!cfg.widgets.apps) {
    host.replaceChildren();
    return;
  }

  const existing = new Map();
  for (const el of host.children) existing.set(el.dataset.hwnd, el);

  const next = [];
  for (const w of apps) {
    const key = String(w.hwnd);
    let el = existing.get(key);
    if (!el) {
      el = document.createElement("button");
      el.className = "pill square open";
      el.dataset.hwnd = key;
      const img = document.createElement("img");
      el.appendChild(img);
      el.onclick = () => invoke("focus_window", { hwnd: w.hwnd }).catch(() => {});
      // Middle click closes, the way the shell's taskbar does. auxclick is the
      // event that actually carries button 1 reliably.
      el.onauxclick = (ev) => {
        if (ev.button !== 1) return;
        ev.preventDefault();
        invoke("close_window", { hwnd: w.hwnd }).catch(() => {});
      };
    }
    const img = el.firstElementChild;
    if (img && img.getAttribute("src") !== w.icon) img.setAttribute("src", w.icon);
    if (el.title !== w.title) el.title = w.title;
    el.classList.toggle("active", w.active);
    next.push(el);
  }
  host.replaceChildren(...next);
}

function applyEdge() {
  const vertical = cfg.edge === "left" || cfg.edge === "right";
  document.body.classList.toggle("vertical", vertical);
}

applyTheme(cfg);
applyEdge();
build();

invoke("list_windows")
  .then((list) => {
    apps = list;
    paintApps();
  })
  .catch(() => {});

listen("apps-changed", (e) => {
  apps = e.payload;
  paintApps();
});

// A second is enough for a clock without seconds too: it keeps the minute
// from lagging by up to a minute after a resume.
setInterval(paintClock, 1000);
setInterval(refreshMic, 2000);

listen("config-changed", (e) => {
  cfg = e.payload;
  applyTheme(cfg);
  applyEdge();
  build();
});
