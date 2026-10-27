//! The bar's rectangle, worked out without touching win32.
//!
//! Split out so it can be tested on any host: this arithmetic is exactly where
//! an off-by-one leaves the bar hanging above the screen edge instead of on
//! it, and that is not something to find out by eye on Windows.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    pub fn horizontal(self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }
}

/// A screen rectangle in virtual-screen coordinates, left/top inclusive and
/// right/bottom exclusive, the way win32 `RECT` means it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// Logical pixels to physical, for a monitor at `dpi`.
///
/// The bar's thickness is configured in logical pixels because that is what
/// the webview lays out in; every win32 call wants physical.
pub fn to_physical(logical: i32, dpi: u32) -> i32 {
    let dpi = if dpi == 0 { 96 } else { dpi };
    let scaled = (logical as f64) * (dpi as f64) / 96.0;
    // Never round down to nothing: a zero-height bar is invisible and reserves
    // no space, which looks exactly like the app having failed to start.
    (scaled.round() as i32).max(1)
}

/// The strip of `screen` that the bar occupies.
///
/// `thickness` is physical pixels and is clamped to the screen, so a silly
/// value cannot produce an inverted rectangle.
pub fn edge_rect(screen: Rect, edge: Edge, thickness: i32) -> Rect {
    let limit = if edge.horizontal() {
        screen.height()
    } else {
        screen.width()
    };
    let t = thickness.clamp(1, limit.max(1));
    match edge {
        Edge::Top => Rect { bottom: screen.top + t, ..screen },
        Edge::Bottom => Rect { top: screen.bottom - t, ..screen },
        Edge::Left => Rect { right: screen.left + t, ..screen },
        Edge::Right => Rect { left: screen.right - t, ..screen },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
    // A monitor to the right of the primary, as the virtual screen lays it out.
    const SECOND: Rect = Rect { left: 1920, top: 0, right: 4480, bottom: 1440 };

    #[test]
    fn bottom_bar_sits_on_the_bottom_edge() {
        let r = edge_rect(FHD, Edge::Bottom, 40);
        assert_eq!(r.bottom, FHD.bottom, "must touch the screen edge");
        assert_eq!(r.top, 1040);
        assert_eq!(r.height(), 40);
        assert_eq!(r.width(), 1920, "must span the screen");
    }

    #[test]
    fn top_bar_sits_on_the_top_edge() {
        let r = edge_rect(FHD, Edge::Top, 40);
        assert_eq!(r.top, FHD.top);
        assert_eq!(r.bottom, 40);
        assert_eq!(r.width(), 1920);
    }

    #[test]
    fn side_bars_span_the_full_height() {
        let l = edge_rect(FHD, Edge::Left, 56);
        assert_eq!((l.left, l.right), (0, 56));
        assert_eq!(l.height(), 1080);

        let r = edge_rect(FHD, Edge::Right, 56);
        assert_eq!((r.left, r.right), (1864, 1920));
        assert_eq!(r.height(), 1080);
    }

    #[test]
    fn a_second_monitor_keeps_its_own_origin() {
        // The bug this guards: using SM_CXSCREEN here would put the bar on the
        // primary monitor whatever screen it belongs to.
        let r = edge_rect(SECOND, Edge::Bottom, 40);
        assert_eq!(r.left, 1920);
        assert_eq!(r.right, 4480);
        assert_eq!(r.bottom, 1440);
        assert_eq!(r.top, 1400);
    }

    #[test]
    fn thickness_cannot_invert_or_vanish() {
        let huge = edge_rect(FHD, Edge::Bottom, 99_999);
        assert_eq!(huge.top, FHD.top, "clamped to the screen, not past it");
        assert!(huge.height() > 0);

        let none = edge_rect(FHD, Edge::Bottom, 0);
        assert_eq!(none.height(), 1, "a zero-height bar would be invisible");
    }

    #[test]
    fn scaling_matches_what_the_webview_lays_out() {
        // 40 logical px of content needs 40 physical at 100%, 60 at 150%, and
        // 80 at 200%; getting this wrong is what left the window a different
        // height from the content inside it.
        assert_eq!(to_physical(40, 96), 40);
        assert_eq!(to_physical(40, 144), 60);
        assert_eq!(to_physical(40, 192), 80);
        assert_eq!(to_physical(40, 120), 50);
    }

    #[test]
    fn unknown_dpi_is_treated_as_unscaled() {
        // GetDpiForWindow answers 0 for a window that has gone.
        assert_eq!(to_physical(40, 0), 40);
    }

    #[test]
    fn scaled_bottom_bar_still_reaches_the_edge() {
        let t = to_physical(40, 144);
        let r = edge_rect(FHD, Edge::Bottom, t);
        assert_eq!(r.bottom, 1080, "the gap under the bar was this going wrong");
        assert_eq!(r.height(), 60);
    }
}
