//! What the bar looks like and which widgets it carries.
//!
//! Stored as JSON next to the crash marker, in `%LOCALAPPDATA%\codabar`. Every
//! field has a default and missing ones are filled in, so a config written by
//! an older build still loads, and a truncated one degrades to defaults instead
//! of leaving the user with no bar.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub fn dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("codabar")
}

pub fn path() -> PathBuf {
    dir().join("config.json")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// Bar fill, as CSS. Alpha here is multiplied by `opacity`.
    pub background: String,
    pub foreground: String,
    pub muted: String,
    pub accent: String,
    /// 0.0 fully see-through, 1.0 solid.
    pub opacity: f32,
    /// Frosted backdrop behind the fill. Costs a little GPU.
    pub blur: bool,
    /// Corner rounding, in px. 0 squares it off against the screen edge.
    pub radius: u32,
    pub font_family: String,
    pub font_size: u32,
    /// Space between widgets, in px.
    pub gap: u32,
    /// Inset from the screen edge, in px. Non-zero floats the bar.
    pub margin: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: "#202020".into(),
            foreground: "#e8e8ee".into(),
            muted: "#8b8b99".into(),
            accent: "#f0be5a".into(),
            opacity: 0.92,
            blur: true,
            radius: 0,
            font_family: "Segoe UI Variable, Segoe UI, system-ui, sans-serif".into(),
            font_size: 12,
            gap: 6,
            margin: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Widgets {
    pub clock: bool,
    /// `true` shows seconds and a longer date.
    pub clock_seconds: bool,
    pub wifi: bool,
    pub mic: bool,
    pub mixer: bool,
    /// The background apps that live in the shell's notification area.
    pub tray: bool,
    pub vpn: bool,
    pub media: bool,
    /// Button that opens Claude Code in a compact window.
    pub claude: bool,
    pub claude_command: String,
    /// The open windows on this monitor, next to the Start button.
    pub apps: bool,
    /// Ceiling on how many are drawn, so a desktop full of windows cannot
    /// push the clock off the bar.
    pub apps_max: usize,
    pub search: bool,
    pub task_view: bool,
    pub volume: bool,
    pub battery: bool,
    /// The sliver at the far end that minimises everything.
    pub show_desktop: bool,
    /// The keyboard language indicator, two lines as the shell draws it.
    pub language: bool,
}

impl Default for Widgets {
    fn default() -> Self {
        Widgets {
            clock: true,
            clock_seconds: false,
            wifi: true,
            mic: true,
            mixer: true,
            tray: true,
            vpn: false,
            media: true,
            claude: true,
            claude_command: "claude".into(),
            apps: true,
            apps_max: 20,
            search: true,
            task_view: true,
            volume: true,
            battery: true,
            show_desktop: true,
            language: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// `top` | `bottom` | `left` | `right`.
    pub edge: String,
    /// How much screen the bar reserves, in px.
    pub thickness: u32,
    /// Hide the shell's own taskbar while we run. Off means we sit alongside it.
    pub replace_shell: bool,
    pub theme: Theme,
    pub widgets: Widgets,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            edge: "bottom".into(),
            thickness: 52,
            replace_shell: true,
            theme: Theme::default(),
            widgets: Widgets::default(),
        }
    }
}

impl Config {
    /// Clamp anything that would make the bar unusable or unreachable.
    pub fn sanitise(&mut self) {
        if !matches!(self.edge.as_str(), "top" | "bottom" | "left" | "right") {
            self.edge = "bottom".into();
        }
        self.thickness = self.thickness.clamp(20, 200);
        self.theme.opacity = self.theme.opacity.clamp(0.0, 1.0);
        self.theme.font_size = self.theme.font_size.clamp(8, 32);
        self.theme.radius = self.theme.radius.min(40);
        self.theme.gap = self.theme.gap.min(40);
        self.theme.margin = self.theme.margin.min(40);
        if self.theme.background.trim().is_empty() {
            self.theme.background = Theme::default().background;
        }
        if self.widgets.claude_command.trim().is_empty() {
            self.widgets.claude_command = Widgets::default().claude_command;
        }
        self.widgets.apps_max = self.widgets.apps_max.clamp(1, 40);
    }

    pub fn load() -> Config {
        let mut cfg = std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str::<Config>(&s).ok())
            .unwrap_or_default();
        cfg.sanitise();
        cfg
    }

    /// Write through a temp file so a crash mid-write cannot truncate the config.
    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(dir())?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = path().with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path())
    }
}
