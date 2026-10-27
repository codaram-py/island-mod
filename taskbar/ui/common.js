// Shared between the bar and the settings window: talking to the backend,
// painting the theme onto :root, and the handful of icons both use.

export const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
export const emit = (evt, payload) => window.__TAURI__.event.emit(evt, payload);
export const listen = (evt, fn) => window.__TAURI__.event.listen(evt, fn);

export async function getConfig() {
  return invoke("get_config");
}

export async function setConfig(cfg) {
  const saved = await invoke("set_config", { next: cfg });
  // The bar and the settings window each hold a copy; whoever writes tells
  // the other, so neither has to poll.
  await emit("config-changed", saved);
  return saved;
}

export function applyTheme(cfg) {
  const t = cfg.theme;
  const r = document.documentElement.style;
  r.setProperty("--bg", t.background);
  r.setProperty("--fg", t.foreground);
  r.setProperty("--muted", t.muted);
  r.setProperty("--accent", t.accent);
  r.setProperty("--opacity", String(t.opacity));
  r.setProperty("--radius", t.radius + "px");
  r.setProperty("--gap", t.gap + "px");
  r.setProperty("--font-size", t.fontSize + "px");
  r.setProperty("--font", t.fontFamily);
  r.setProperty("--margin", t.margin + "px");
  for (const el of document.querySelectorAll(".surface")) {
    el.classList.toggle("blur", !!t.blur);
  }
}

// Lucide paths, inlined: the bar must paint before anything over the network.
const PATHS = {
  mic: ["M12 19v3", "M19 10v2a7 7 0 0 1-14 0v-2", "M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3z"],
  micOff: ["m2 2 20 20", "M18.89 13.23A7 7 0 0 0 19 12v-2", "M5 10v2a7 7 0 0 0 12 5", "M15 9.34V5a3 3 0 0 0-5.68-1.33", "M9 9v3a3 3 0 0 0 5.12 2.12", "M12 19v3"],
  settings: ["M20 7h-9", "M14 17H5", "M17 14a3 3 0 1 0 0 6 3 3 0 0 0 0-6z", "M7 4a3 3 0 1 0 0 6 3 3 0 0 0 0-6z"],
  terminal: ["m4 17 6-6-6-6", "M12 19h8"],
  // Four wi-fi strengths: the arcs drop away as the signal does, so the shape
  // itself carries the reading without needing a number.
  wifi3: ["M12 20h.01", "M5 12.9a10 10 0 0 1 14 0", "M8.5 16.4a5 5 0 0 1 7 0", "M2 8.8a15 15 0 0 1 20 0"],
  wifi2: ["M12 20h.01", "M5 12.9a10 10 0 0 1 14 0", "M8.5 16.4a5 5 0 0 1 7 0"],
  wifi1: ["M12 20h.01", "M8.5 16.4a5 5 0 0 1 7 0"],
  wifi0: ["M12 20h.01", "m2 2 20 20", "M8.5 16.4a5 5 0 0 1 7 0"],
  wired: ["M6.5 7h11", "M12 7v4", "M4 11h16v4H4z", "M8 19v-4", "M16 19v-4"],
  // Drawn in icon() rather than here: it is filled, not stroked.
  start: null,
};

export function icon(name) {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("class", "icon");

  // Windows flag: four filled tiles in the Microsoft blue, with a 1-px gap
  // and a hair of asymmetry to echo the trapezoids of the real glyph.
  if (name === "start") {
    svg.setAttribute("fill", "#0078d4");
    svg.setAttribute("stroke", "none");
    for (const d of [
      "M3 4.5l8-1.5v8H3z",
      "M12 2.75l9-1.75v10h-9z",
      "M3 13h8v8.5l-8-1.5z",
      "M12 13h9v10.25l-9-1.75z",
    ]) {
      const p = document.createElementNS(ns, "path");
      p.setAttribute("d", d);
      svg.appendChild(p);
    }
    return svg;
  }

  for (const d of PATHS[name] || []) {
    const p = document.createElementNS(ns, "path");
    p.setAttribute("d", d);
    svg.appendChild(p);
  }
  return svg;
}
