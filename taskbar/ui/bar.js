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

// ---- network ---------------------------------------------------------------
let netBtn = null;

function paintNet(info) {
  if (!netBtn) return;
  if (!info || info.kind === "none") {
    netBtn.replaceChildren(icon("wifi0"));
    netBtn.title = "Sem conexão";
    netBtn.classList.add("alert");
    return;
  }
  netBtn.classList.remove("alert");
  if (info.kind === "wired") {
    netBtn.replaceChildren(icon("wired"));
    netBtn.title = info.online ? "Rede cabeada" : "Rede cabeada · sem internet";
    return;
  }
  // Thirds of the quality scale, so a bar is lost roughly every 33 points.
  const bars = info.quality >= 67 ? 3 : info.quality >= 34 ? 2 : 1;
  netBtn.replaceChildren(icon("wifi" + bars));
  netBtn.title =
    (info.ssid || "Wi-Fi") +
    " · " +
    info.quality +
    "%" +
    (info.online ? "" : " · sem internet");
}

async function refreshNet() {
  if (!netBtn) return;
  paintNet(await invoke("net_status").catch(() => null));
}

// ---- volume ----------------------------------------------------------------
// Click mutes, wheel nudges the level: the two things the shell's own icon
// does without opening its flyout.
let volBtn = null;

function volIcon(v) {
  if (v.muted) return "volMute";
  return v.level >= 60 ? "vol2" : v.level >= 15 ? "vol1" : "vol0";
}

function paintVolume(v) {
  if (!volBtn) return;
  volBtn.classList.toggle("hidden", !v);
  if (!v) return;
  volBtn.replaceChildren(icon(volIcon(v)));
  volBtn.classList.toggle("alert", v.muted);
  volBtn.title = v.muted ? "Som mudo — clique para ativar" : "Volume " + v.level + "% — roda para ajustar";
  volBtn.dataset.level = String(v.level);
}

async function refreshVolume() {
  if (!volBtn) return;
  paintVolume(await invoke("volume").catch(() => null));
}

// ---- battery ---------------------------------------------------------------
let batBtn = null;

function paintBattery(b) {
  if (!batBtn) return;
  // A desktop has none, and a widget that never says anything is just noise.
  batBtn.classList.toggle("hidden", !b);
  if (!b) return;
  const fill = document.createElement("span");
  fill.className = "bat";
  fill.style.setProperty("--pct", b.percent + "%");
  const glyph = icon(b.charging ? "bolt" : "battery");
  batBtn.replaceChildren(glyph, fill);
  batBtn.classList.toggle("alert", b.percent <= 15 && !b.charging);
  const hours = b.secondsLeft ? Math.floor(b.secondsLeft / 3600) : null;
  const mins = b.secondsLeft ? Math.round((b.secondsLeft % 3600) / 60) : null;
  batBtn.title =
    b.percent +
    "%" +
    (b.charging ? " · carregando" : hours !== null ? ` · ${hours}h${String(mins).padStart(2, "0")} restantes` : "");
}

async function refreshBattery() {
  if (!batBtn) return;
  paintBattery(await invoke("battery").catch(() => null));
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
  netBtn = null;
  volBtn = null;
  batBtn = null;
  clockEl = null;

  // ---- center: Start + apps ------------------------------------------------
  const start = pill(center, "start", "Menu Iniciar");
  start.classList.add("square");
  start.appendChild(icon("start"));
  start.onclick = () => invoke("open_start_menu").catch(() => {});

  if (cfg.widgets.search) {
    const b = pill(center, "search", "Pesquisar");
    b.classList.add("square");
    b.appendChild(icon("search"));
    b.onclick = () => invoke("open_search").catch(() => {});
  }

  if (cfg.widgets.taskView) {
    const b = pill(center, "taskview", "Visão de tarefas");
    b.classList.add("square");
    b.appendChild(icon("taskView"));
    b.onclick = () => invoke("open_task_view").catch(() => {});
  }
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

  if (cfg.widgets.volume) {
    volBtn = pill(right, "volume", "Volume");
    volBtn.classList.add("square");
    volBtn.onclick = async () => paintVolume(await invoke("toggle_speakers"));
    volBtn.onwheel = (ev) => {
      ev.preventDefault();
      const now = Number(volBtn.dataset.level || 0);
      const next = Math.max(0, Math.min(100, now + (ev.deltaY < 0 ? 4 : -4)));
      if (next === now) return;
      volBtn.dataset.level = String(next);
      invoke("set_volume", { level: next }).then(paintVolume).catch(() => {});
    };
    refreshVolume();
  }

  if (cfg.widgets.wifi) {
    netBtn = pill(right, "net", "Rede");
    netBtn.classList.add("square");
    // The quickest route to the thing you actually want when you click it.
    netBtn.onclick = () => invoke("open_network_settings").catch(() => {});
    refreshNet();
  }

  if (cfg.widgets.mic) {
    micBtn = pill(right, "mic", "Microfone");
    micBtn.classList.add("square");
    micBtn.onclick = async () => paintMic(await invoke("toggle_mic"));
    refreshMic();
  }

  if (cfg.widgets.battery) {
    batBtn = pill(right, "battery", "Bateria");
    batBtn.classList.add("square");
    refreshBattery();
  }

  const gear = pill(right, "settings", "Ajustes do codabar");
  gear.classList.add("square");
  gear.appendChild(icon("settings"));
  gear.onclick = () => invoke("open_settings").catch(() => {});

  if (cfg.widgets.clock) {
    clockEl = pill(right, "clock", "");
    paintClock();
  }

  if (cfg.widgets.showDesktop) {
    // The sliver at the very end, exactly as the shell's taskbar has it.
    const b = pill(right, "showdesktop", "Mostrar a área de trabalho");
    b.className = "sliver";
    b.onclick = () => invoke("show_desktop").catch(() => {});
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
      el.onclick = () => invoke("focus_window", { hwnd: w.hwnd }).catch(() => {});
      // Middle click closes, the way the shell's taskbar does. auxclick is the
      // event that actually carries button 1 reliably.
      el.onauxclick = (ev) => {
        if (ev.button !== 1) return;
        ev.preventDefault();
        invoke("close_window", { hwnd: w.hwnd }).catch(() => {});
      };
    }
    // An empty src makes the webview re-request the page itself, so a window
    // whose icon could not be read gets its initial instead of a blank tile.
    if (el.dataset.icon !== w.icon) {
      el.dataset.icon = w.icon;
      if (w.icon) {
        const img = document.createElement("img");
        img.src = w.icon;
        img.alt = "";
        el.replaceChildren(img);
      } else {
        const span = document.createElement("span");
        span.className = "initial";
        span.textContent = (w.title.trim()[0] || "?").toUpperCase();
        el.replaceChildren(span);
      }
    }
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
// Signal strength drifts slowly and the WLAN query is the dearest of the
// three, so it runs a good deal less often.
setInterval(refreshNet, 5000);
setInterval(refreshVolume, 2000);
// Charge moves slowly enough that anything faster is wasted wakeups.
setInterval(refreshBattery, 30000);

listen("config-changed", (e) => {
  cfg = e.payload;
  applyTheme(cfg);
  applyEdge();
  build();
});
