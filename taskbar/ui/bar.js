import { invoke, listen, getConfig, applyTheme, icon } from "./common.js";

const right = document.getElementById("right");
let cfg = await getConfig();

function pill(id, title) {
  const b = document.createElement("button");
  b.className = "pill";
  b.id = id;
  b.title = title;
  right.appendChild(b);
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
function build() {
  right.replaceChildren();
  micBtn = null;
  clockEl = null;

  if (cfg.widgets.claude) {
    const b = pill("claude", "Abrir o Claude Code");
    b.appendChild(icon("terminal"));
    b.onclick = () => invoke("launch_claude").catch(() => {});
  }

  if (cfg.widgets.mic) {
    micBtn = pill("mic", "Microfone");
    micBtn.onclick = async () => paintMic(await invoke("toggle_mic"));
    refreshMic();
  }

  const gear = pill("settings", "Ajustes do codabar");
  gear.appendChild(icon("settings"));
  gear.onclick = () => invoke("open_settings").catch(() => {});

  if (cfg.widgets.clock) {
    clockEl = pill("clock", "");
    paintClock();
  }
}

function applyEdge() {
  const vertical = cfg.edge === "left" || cfg.edge === "right";
  document.body.classList.toggle("vertical", vertical);
}

applyTheme(cfg);
applyEdge();
build();

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
