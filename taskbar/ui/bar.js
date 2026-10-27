import { invoke, listen, getConfig, applyTheme, icon } from "./common.js";

const center = document.getElementById("center");
const right = document.getElementById("right");
const sliver = document.getElementById("showdesktop");
let cfg = await getConfig();

const el = (tag, cls, props) =>
  Object.assign(document.createElement(tag), { className: cls || "" }, props);

function tile(host, id, title) {
  const b = el("button", "tile", { id, title });
  host.appendChild(b);
  return b;
}

function rule(host) {
  host.appendChild(el("div", "rule"));
}

// Live nodes the refreshers write into; cleared on every rebuild so a stale
// one can never be painted after the layout changed under it.
let volBtn = null;
let micBtn = null;
let netBtn = null;
let batBtn = null;
let clockEl = null;
let langEl = null;

// ---- volume ----------------------------------------------------------------
// Click mutes, wheel nudges the level: the two things the shell's own icon
// does without opening its flyout.
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
  volBtn.title = v.muted
    ? "Som mudo — clique para ativar"
    : "Volume " + v.level + "% — roda para ajustar";
  volBtn.dataset.level = String(v.level);
}

async function refreshVolume() {
  if (!volBtn) return;
  paintVolume(await invoke("volume").catch(() => null));
}

// ---- microphone ------------------------------------------------------------
function paintMic(muted) {
  if (!micBtn) return;
  micBtn.classList.toggle("hidden", muted === null);
  if (muted === null) return;
  micBtn.replaceChildren(icon(muted ? "micOff" : "mic"));
  micBtn.classList.toggle("alert", muted);
  micBtn.title = muted
    ? "Microfone mudo — clique para ativar"
    : "Microfone ativo — clique para mutar";
}

async function refreshMic() {
  if (!micBtn) return;
  paintMic(await invoke("mic_muted").catch(() => null));
}

// ---- network ---------------------------------------------------------------
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
  // Thirds of the quality scale, so an arc is lost roughly every 33 points.
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

// ---- battery ---------------------------------------------------------------
function paintBattery(b) {
  if (!batBtn) return;
  // A desktop has none, and a widget that never says anything is just noise.
  batBtn.classList.toggle("hidden", !b);
  if (!b) return;
  batBtn.replaceChildren(icon(b.charging ? "bolt" : "battery"));
  batBtn.classList.toggle("alert", b.percent <= 15 && !b.charging);
  const hours = b.secondsLeft ? Math.floor(b.secondsLeft / 3600) : null;
  const mins = b.secondsLeft ? Math.round((b.secondsLeft % 3600) / 60) : null;
  batBtn.title =
    b.percent +
    "%" +
    (b.charging
      ? " · carregando"
      : hours !== null
        ? ` · ${hours}h${String(mins).padStart(2, "0")} restantes`
        : "");
}

async function refreshBattery() {
  if (!batBtn) return;
  paintBattery(await invoke("battery").catch(() => null));
}

// ---- keyboard language -----------------------------------------------------
function paintLanguage(l) {
  if (!langEl) return;
  langEl.classList.toggle("hidden", !l);
  if (!l) return;
  langEl.replaceChildren(
    el("span", "lang", { textContent: l.language }),
    el("span", "klid", { textContent: l.layout }),
  );
  langEl.title = l.language + " · " + l.layout;
}

async function refreshLanguage() {
  if (!langEl) return;
  paintLanguage(await invoke("keyboard_layout").catch(() => null));
}

// ---- clock -----------------------------------------------------------------
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
    el("span", "time", { textContent: time }),
    el("span", "date", { textContent: date }),
  );
  clockEl.title = now.toLocaleString();
}

// ---- open windows ----------------------------------------------------------
// The backend pushes the list once a second, but only when it differs. Nodes
// are reused by hwnd so an icon never blinks: a refresh that only moved the
// focus touches nothing at all.
let apps = [];

function paintApps() {
  const host = document.getElementById("apps");
  if (!host) return;
  if (!cfg.widgets.apps) {
    host.replaceChildren();
    return;
  }

  const existing = new Map();
  for (const node of host.children) existing.set(node.dataset.hwnd, node);

  const next = [];
  for (const w of apps) {
    const key = String(w.hwnd);
    let node = existing.get(key);
    if (!node) {
      node = el("button", "tile");
      node.dataset.hwnd = key;
      node.onclick = () => invoke("focus_window", { hwnd: w.hwnd }).catch(() => {});
      // Middle click closes, the way the shell's taskbar does. auxclick is the
      // event that actually carries button 1 reliably.
      node.onauxclick = (ev) => {
        if (ev.button !== 1) return;
        ev.preventDefault();
        invoke("close_window", { hwnd: w.hwnd }).catch(() => {});
      };
    }
    // An empty src makes the webview re-request the page itself, so a window
    // whose icon could not be read gets its initial instead of a blank tile.
    if (node.dataset.icon !== w.icon) {
      node.dataset.icon = w.icon;
      if (w.icon) {
        node.replaceChildren(el("img", "", { src: w.icon, alt: "" }));
      } else {
        node.replaceChildren(
          el("span", "initial", {
            textContent: (w.title.trim()[0] || "?").toUpperCase(),
          }),
        );
      }
    }
    if (node.title !== w.title) node.title = w.title;
    next.push(node);
  }
  host.replaceChildren(...next);
}

// ---- layout ----------------------------------------------------------------
// Rebuilt whole rather than diffed: a few dozen nodes, and it keeps the widget
// order honest against the config.
function build() {
  center.replaceChildren();
  right.replaceChildren();
  volBtn = micBtn = netBtn = batBtn = clockEl = langEl = null;

  // ---- centre: Start, then the open windows --------------------------------
  const start = tile(center, "start", "Menu Iniciar");
  start.appendChild(icon("start"));
  start.onclick = () => invoke("open_start_menu").catch(() => {});

  if (cfg.widgets.search) {
    const b = tile(center, "search", "Pesquisar");
    b.appendChild(icon("search"));
    b.onclick = () => invoke("open_search").catch(() => {});
  }

  if (cfg.widgets.taskView) {
    const b = tile(center, "taskview", "Visão de tarefas");
    b.appendChild(icon("taskView"));
    b.onclick = () => invoke("open_task_view").catch(() => {});
  }

  const appsHost = el("div", "", { id: "apps" });
  center.appendChild(appsHost);
  paintApps();

  // ---- right: tray, language, status, clock --------------------------------
  // The tray is not mirrored yet, so its toggle and slots are left out rather
  // than drawn empty.

  if (cfg.widgets.language) {
    langEl = el("button", "group", { id: "language" });
    langEl.onclick = () => invoke("cycle_layout").catch(() => {});
    right.appendChild(langEl);
    refreshLanguage();
    rule(right);
  }

  const status = el("div", "group", { id: "status" });
  right.appendChild(status);

  if (cfg.widgets.claude) {
    const b = el("button", "", { title: "Abrir o Claude Code" });
    b.appendChild(icon("terminal"));
    b.onclick = () => invoke("launch_claude").catch(() => {});
    status.appendChild(b);
  }

  // Audio over microphone, as the shell stacks them.
  const soundStack = el("div", "stack");
  status.appendChild(soundStack);
  if (cfg.widgets.volume) {
    volBtn = el("button", "", { title: "Volume" });
    volBtn.onclick = async () => paintVolume(await invoke("toggle_speakers"));
    volBtn.onwheel = (ev) => {
      ev.preventDefault();
      const now = Number(volBtn.dataset.level || 0);
      const next = Math.max(0, Math.min(100, now + (ev.deltaY < 0 ? 4 : -4)));
      if (next === now) return;
      volBtn.dataset.level = String(next);
      invoke("set_volume", { level: next }).then(paintVolume).catch(() => {});
    };
    soundStack.appendChild(volBtn);
    refreshVolume();
  }
  if (cfg.widgets.mic) {
    micBtn = el("button", "", { title: "Microfone" });
    micBtn.onclick = async () => paintMic(await invoke("toggle_mic"));
    soundStack.appendChild(micBtn);
    refreshMic();
  }

  // Network over bluetooth. Bluetooth has no backend yet, so the slot is the
  // wi-fi glyph alone rather than a mark that means nothing.
  const netStack = el("div", "stack");
  status.appendChild(netStack);
  if (cfg.widgets.wifi) {
    netBtn = el("button", "", { title: "Rede" });
    netBtn.onclick = () => invoke("open_network_settings").catch(() => {});
    netStack.appendChild(netBtn);
    refreshNet();
  }
  if (cfg.widgets.battery) {
    batBtn = el("button", "", { title: "Bateria" });
    netStack.appendChild(batBtn);
    refreshBattery();
  }

  const gear = el("button", "", { title: "Ajustes do codabar" });
  gear.appendChild(icon("settings"));
  gear.onclick = () => invoke("open_settings").catch(() => {});
  status.appendChild(gear);

  if (cfg.widgets.clock) {
    rule(right);
    clockEl = el("button", "group", { id: "clock" });
    right.appendChild(clockEl);
    paintClock();
  }

  sliver.classList.toggle("hidden", !cfg.widgets.showDesktop);
}

function applyEdge() {
  const vertical = cfg.edge === "left" || cfg.edge === "right";
  document.body.classList.toggle("vertical", vertical);
}

sliver.onclick = () => invoke("show_desktop").catch(() => {});

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
setInterval(refreshVolume, 2000);
// Signal strength drifts slowly and the WLAN query is the dearest of these.
setInterval(refreshNet, 5000);
setInterval(refreshLanguage, 2000);
// Charge moves slowly enough that anything faster is wasted wakeups.
setInterval(refreshBattery, 30000);

listen("config-changed", (e) => {
  cfg = e.payload;
  applyTheme(cfg);
  applyEdge();
  build();
});
