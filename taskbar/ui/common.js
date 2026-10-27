// Shared between the bar and the settings window: talking to the backend,
// painting the theme onto :root, and the icon set.

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

// Each entry is its own viewBox and markup, because these are not one family:
// the tray glyphs are drawn on a 16-unit grid, the chevron on a 10-unit one,
// and the Windows flag on 20. Sizes are the mockup's.
//
// `currentColor` wherever the glyph should follow the widget's state (muted,
// alert); a literal colour where the glyph is the brand, like the flag or the
// bluetooth mark.
const ICONS = {
  start: {
    box: "0 0 20 20",
    size: 20,
    svg: `<rect x="1" y="1" width="8.5" height="8.5" rx="1.5" fill="#0078D4"/>
          <rect x="10.5" y="1" width="8.5" height="8.5" rx="1.5" fill="#0078D4"/>
          <rect x="1" y="10.5" width="8.5" height="8.5" rx="1.5" fill="#0078D4"/>
          <rect x="10.5" y="10.5" width="8.5" height="8.5" rx="1.5" fill="#0078D4"/>`,
  },
  search: {
    box: "0 0 24 24",
    size: 20,
    svg: `<circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="1.6" fill="none"/>
          <path d="m20 20-4.2-4.2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" fill="none"/>`,
  },
  taskView: {
    box: "0 0 24 24",
    size: 20,
    svg: `<rect x="3" y="5" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <rect x="13" y="5" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <rect x="3" y="13" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <rect x="13" y="13" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>`,
  },
  chevronUp: {
    box: "0 0 10 10",
    size: 10,
    svg: `<path d="M2 7L5 3L8 7" stroke="#aaa" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>`,
  },
  // Standalone, in the status cluster.
  terminal: {
    box: "0 0 16 16",
    size: 15,
    svg: `<rect x="1" y="2" width="14" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.2"/>
          <path d="M4 6L7 8.5L4 11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          <line x1="8" y1="11" x2="12" y2="11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>`,
  },
  // Speaker, with the arcs dropping away as the level does.
  vol2: {
    box: "0 0 16 16",
    size: 13,
    svg: `<path d="M2 6H5L9 3V13L5 10H2V6Z" fill="currentColor"/>
          <path d="M11 5.5C11 5.5 12.5 7 12.5 8C12.5 9 11 10.5 11 10.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>
          <path d="M12 3.5C12 3.5 14.5 6 14.5 8C14.5 10 12 12.5 12 12.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>`,
  },
  vol1: {
    box: "0 0 16 16",
    size: 13,
    svg: `<path d="M2 6H5L9 3V13L5 10H2V6Z" fill="currentColor"/>
          <path d="M11 5.5C11 5.5 12.5 7 12.5 8C12.5 9 11 10.5 11 10.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" fill="none"/>`,
  },
  vol0: {
    box: "0 0 16 16",
    size: 13,
    svg: `<path d="M2 6H5L9 3V13L5 10H2V6Z" fill="currentColor"/>`,
  },
  volMute: {
    box: "0 0 16 16",
    size: 13,
    svg: `<path d="M2 6H5L9 3V13L5 10H2V6Z" fill="currentColor"/>
          <line x1="11" y1="5.5" x2="15" y2="10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <line x1="15" y1="5.5" x2="11" y2="10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>`,
  },
  mic: {
    box: "0 0 16 16",
    size: 10,
    svg: `<rect x="5.5" y="1" width="5" height="9" rx="2.5" fill="currentColor"/>
          <path d="M3.5 7.5C3.5 7.5 3.5 11 8 11C12.5 11 12.5 7.5 12.5 7.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <line x1="8" y1="11" x2="8" y2="13.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <line x1="5.5" y1="13.5" x2="10.5" y2="13.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>`,
  },
  micOff: {
    box: "0 0 16 16",
    size: 10,
    svg: `<rect x="5.5" y="1" width="5" height="9" rx="2.5" fill="currentColor"/>
          <path d="M3.5 7.5C3.5 7.5 3.5 11 8 11C12.5 11 12.5 7.5 12.5 7.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <line x1="8" y1="11" x2="8" y2="13.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <line x1="2" y1="2" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>`,
  },
  // Three arcs and a dot; arcs drop away with the signal.
  wifi3: {
    box: "0 0 16 16",
    size: 15,
    svg: `<path d="M1 5.5C1 5.5 4 2 8 2C12 2 15 5.5 15 5.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <path d="M3 8C3 8 5 5.5 8 5.5C11 5.5 13 8 13 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <path d="M5.5 10.5C5.5 10.5 6.5 9 8 9C9.5 9 10.5 10.5 10.5 10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <circle cx="8" cy="13" r="1" fill="currentColor"/>`,
  },
  wifi2: {
    box: "0 0 16 16",
    size: 15,
    svg: `<path d="M3 8C3 8 5 5.5 8 5.5C11 5.5 13 8 13 8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <path d="M5.5 10.5C5.5 10.5 6.5 9 8 9C9.5 9 10.5 10.5 10.5 10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <circle cx="8" cy="13" r="1" fill="currentColor"/>`,
  },
  wifi1: {
    box: "0 0 16 16",
    size: 15,
    svg: `<path d="M5.5 10.5C5.5 10.5 6.5 9 8 9C9.5 9 10.5 10.5 10.5 10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <circle cx="8" cy="13" r="1" fill="currentColor"/>`,
  },
  wifi0: {
    box: "0 0 16 16",
    size: 15,
    svg: `<path d="M5.5 10.5C5.5 10.5 6.5 9 8 9C9.5 9 10.5 10.5 10.5 10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
          <line x1="2" y1="2" x2="14" y2="14" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>`,
  },
  wired: {
    box: "0 0 16 16",
    size: 15,
    svg: `<rect x="2" y="6" width="12" height="6" rx="1.5" stroke="currentColor" stroke-width="1.3" fill="none"/>
          <line x1="8" y1="3" x2="8" y2="6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          <line x1="4" y1="3" x2="12" y2="3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>`,
  },
  bluetooth: {
    box: "0 0 16 16",
    size: 9,
    svg: `<path d="M4,5 L12,11 L8,15 V1 L12,5 L4,11" stroke="#4A9FD9" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>`,
  },
  battery: {
    box: "0 0 16 16",
    size: 13,
    svg: `<rect x="1" y="5" width="12" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <rect x="14" y="7" width="1.5" height="2" rx="0.6" fill="currentColor"/>`,
  },
  bolt: {
    box: "0 0 16 16",
    size: 13,
    svg: `<rect x="1" y="5" width="12" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <rect x="14" y="7" width="1.5" height="2" rx="0.6" fill="currentColor"/>
          <path d="M7.5 5.5L5.5 8.4h2l-1 2.3" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round" fill="none"/>`,
  },
  settings: {
    box: "0 0 16 16",
    size: 14,
    svg: `<line x1="13.5" y1="4.5" x2="7" y2="4.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          <line x1="9" y1="11.5" x2="2.5" y2="11.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          <circle cx="4.5" cy="4.5" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <circle cx="11.5" cy="11.5" r="2" stroke="currentColor" stroke-width="1.2" fill="none"/>`,
  },
};

/// Build an icon. `size` overrides the entry's own, for the odd reuse.
export function icon(name, size) {
  const spec = ICONS[name];
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  if (!spec) return svg;
  const px = size ?? spec.size;
  svg.setAttribute("viewBox", spec.box);
  svg.setAttribute("width", px);
  svg.setAttribute("height", px);
  svg.setAttribute("fill", "none");
  // Static markup authored here, never anything that came from a window title
  // or another process.
  svg.innerHTML = spec.svg;
  return svg;
}
