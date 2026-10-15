//! Where the island rests on a screen (docs/09-DEMO-FINALE.md, section 3): magnets,
//! momentum, the spring that carries it, the pill's anchor in its window and the way the
//! island opens. Pure geometry in physical pixels of the virtual screen, ported from
//! `moveIsland`, `magnets` and `dropIsland` in reference/ilot-demo-finale.html; the
//! window code (window.rs) applies it.

use crate::players::Area;
use serde::Serialize;

/// Logical size of the island's window: the opened island, the panel card and their shadows.
pub const WINDOW_WIDTH: f64 = 980.0;
pub const WINDOW_HEIGHT: f64 = 720.0;
/// Half width of the folded pill (220 px) until the interface sends its shape.
pub const PILL_HALF_WIDTH: f64 = 110.0;
/// Distance between the pill's center and the screen edge it rests against.
const EDGE: f64 = 32.0;
/// Closest the pill's center comes to the top or bottom edge.
const EDGE_MIN: f64 = 28.0;
/// Gap between the pill and a side edge.
const SIDE_GAP: f64 = 8.0;
/// Gap between the pill and a side edge at a magnet.
const MAGNET_GAP: f64 = 16.0;
/// A magnet this close to the drop point catches the island.
const MAGNET_REACH: f64 = 110.0;
/// Momentum: the drop point goes on along the release speed for this many seconds.
const THROW_S: f64 = 0.12;
/// Share of the release speed the spring starts with.
pub const CARRY: f64 = 0.6;
/// Release speed when two moves share the same instant (the demo's 16 ms).
const SAME_INSTANT_S: f64 = 0.016;

/// A point in physical pixels of the virtual screen (or a speed in pixels per second).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn distance(self, other: Point) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

/// A monitor as the island sees it: its name, work area and scale factor.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub name: String,
    pub work: Area,
    pub scale: f64,
}

fn edges(work: Area) -> (f64, f64, f64, f64) {
    let left = f64::from(work.x);
    let top = f64::from(work.y);
    (left, top, left + f64::from(work.width), top + f64::from(work.height))
}

/// `value` within `[low, high]`; the middle when the range is empty.
fn within(value: f64, low: f64, high: f64) -> f64 {
    if low > high { (low + high) / 2.0 } else { value.clamp(low, high) }
}

/// Keeps the pill whole on the work area: a small gap from the sides, 28 px from the
/// top and bottom. `half_width` is the pill's, in logical pixels.
pub fn clamp_center(screen: &Screen, half_width: f64, p: Point) -> Point {
    let (left, top, right, bottom) = edges(screen.work);
    let side = (half_width + SIDE_GAP) * screen.scale;
    let edge = EDGE_MIN * screen.scale;
    Point::new(within(p.x, left + side, right - side), within(p.y, top + edge, bottom - edge))
}

/// The eight resting points: top, bottom, left and right, corners included. The first
/// one is the top center, where the island starts.
pub fn magnets(screen: &Screen, half_width: f64) -> [Point; 8] {
    let (left, top, right, bottom) = edges(screen.work);
    let side = (half_width + MAGNET_GAP) * screen.scale;
    let edge = EDGE * screen.scale;
    let (cx, cy) = ((left + right) / 2.0, (top + bottom) / 2.0);
    let (west, east) = (within(left + side, left, cx), within(right - side, cx, right));
    [
        Point::new(cx, top + edge),
        Point::new(west, top + edge),
        Point::new(east, top + edge),
        Point::new(cx, bottom - edge),
        Point::new(west, bottom - edge),
        Point::new(east, bottom - edge),
        Point::new(west, cy),
        Point::new(east, cy),
    ]
}

/// True when the pill rests against the top edge of the screen (`anchor_y`, its center
/// in the window, no lower than a resting pill's): only then does the strip above it
/// belong to the island.
pub fn against_top(anchor_y: f64) -> bool {
    anchor_y <= EDGE + 0.5
}

pub fn top_center(screen: &Screen, half_width: f64) -> Point {
    magnets(screen, half_width)[0]
}

/// Where a thrown island lands: a little further along its speed, kept on the screen,
/// then caught by the nearest magnet when it is close enough (always when `always`).
pub fn drop_target(screen: &Screen, half_width: f64, at: Point, speed: Point, always: bool) -> Point {
    let thrown = clamp_center(screen, half_width, Point::new(at.x + speed.x * THROW_S, at.y + speed.y * THROW_S));
    let nearest = magnets(screen, half_width)
        .into_iter()
        .min_by(|a, b| a.distance(thrown).total_cmp(&b.distance(thrown)))
        .unwrap_or(thrown);
    if always || nearest.distance(thrown) < MAGNET_REACH * screen.scale { nearest } else { thrown }
}

/// Speed at release, from the first and last of the recent moves (seconds, position).
pub fn release_speed(moves: &[(f64, Point)]) -> Point {
    let (Some(&(t0, a)), Some(&(t1, b))) = (moves.first(), moves.last()) else { return Point::default() };
    if moves.len() < 2 {
        return Point::default();
    }
    let seconds = if t1 > t0 { t1 - t0 } else { SAME_INSTANT_S };
    Point::new((b.x - a.x) / seconds, (b.y - a.y) / seconds)
}

/// In the top half of the screen the island opens downward, in the bottom half upward.
pub fn opens_down(screen: &Screen, y: f64) -> bool {
    let (_, top, _, bottom) = edges(screen.work);
    y < (top + bottom) / 2.0
}

/// The window around a resting pill: its top-left corner (physical), the pill's center
/// in it (logical, sent to the interface) and the way the island opens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub window: (i32, i32),
    pub anchor: Point,
    pub opens_down: bool,
}

/// Start of a span placed at `start`, kept inside the area; a span longer than the
/// area is centered on it.
fn fit(start: f64, length: f64, area_start: f64, area_end: f64) -> f64 {
    let last = area_end - length;
    if last < area_start { (area_start + last) / 2.0 } else { start.clamp(area_start, last) }
}

/// Places the window so that it stays on the work area: the pill sits at the top of the
/// window when the island opens downward, at the bottom otherwise, centered across; near
/// an edge the window stops and the anchor moves inside it instead.
pub fn frame(screen: &Screen, center: Point) -> Frame {
    let (left, top, right, bottom) = edges(screen.work);
    let scale = screen.scale;
    let down = opens_down(screen, center.y);
    let anchor_y = if down { EDGE } else { WINDOW_HEIGHT - EDGE };
    let x = fit(center.x - WINDOW_WIDTH / 2.0 * scale, WINDOW_WIDTH * scale, left, right).round();
    let y = if WINDOW_HEIGHT * scale > bottom - top {
        if down { top } else { bottom - WINDOW_HEIGHT * scale }
    } else {
        fit(center.y - anchor_y * scale, WINDOW_HEIGHT * scale, top, bottom)
    }
    .round();
    Frame {
        window: (x as i32, y as i32),
        anchor: Point::new((center.x - x) / scale, (center.y - y) / scale),
        opens_down: down,
    }
}

/// Top-left corner of the window that puts the anchor on `center` (window scale `scale`).
pub fn window_at(center: Point, anchor: Point, scale: f64) -> (i32, i32) {
    ((center.x - anchor.x * scale).round() as i32, (center.y - anchor.y * scale).round() as i32)
}

/// The pill's center as a share of the work area, kept across resolutions.
pub fn to_share(work: Area, p: Point) -> Point {
    let (left, top, _, _) = edges(work);
    Point::new((p.x - left) / f64::from(work.width.max(1)), (p.y - top) / f64::from(work.height.max(1)))
}

/// Back from a share of the work area; a damaged share gives nothing.
pub fn from_share(work: Area, share: Point) -> Option<Point> {
    if !share.x.is_finite() || !share.y.is_finite() {
        return None;
    }
    let (left, top, _, _) = edges(work);
    Some(Point::new(
        left + share.x.clamp(0.0, 1.0) * f64::from(work.width),
        top + share.y.clamp(0.0, 1.0) * f64::from(work.height),
    ))
}

/// The demo's interruptible spring (src/lib/spring.ts): response 0.34 s, damping 0.82,
/// integrated in 4 ms steps. A new target never makes it jump.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f64,
    pub target: f64,
    pub velocity: f64,
}

const RESPONSE_S: f64 = 0.34;
const DAMPING: f64 = 0.82;
const STEP_S: f64 = 0.004;
/// Close enough to rest, in pixels and pixels per second.
const REST_DISTANCE: f64 = 0.01;
const REST_SPEED: f64 = 0.01;

impl Spring {
    pub fn new(value: f64, target: f64, velocity: f64) -> Self {
        Self { value, target, velocity }
    }

    /// Advances by `seconds`; returns true while it still moves.
    pub fn step(&mut self, seconds: f64) -> bool {
        let stiffness = (2.0 * std::f64::consts::PI / RESPONSE_S).powi(2);
        let friction = 2.0 * DAMPING * stiffness.sqrt();
        let steps = (seconds / STEP_S).ceil().max(1.0);
        let h = seconds / steps;
        for _ in 0..steps as u32 {
            let acceleration = -stiffness * (self.value - self.target) - friction * self.velocity;
            self.velocity += acceleration * h;
            self.value += self.velocity * h;
        }
        if (self.value - self.target).abs() < REST_DISTANCE && self.velocity.abs() < REST_SPEED {
            self.value = self.target;
            self.velocity = 0.0;
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: i32, y: i32, width: i32, height: i32, scale: f64) -> Screen {
        Screen { name: r"\\.\DISPLAY1".into(), work: Area { x, y, width, height }, scale }
    }

    /// 1920 × 1080 with a 40 px taskbar at the bottom.
    fn main_screen() -> Screen {
        screen(0, 0, 1920, 1040, 1.0)
    }

    #[test]
    fn eight_magnets_on_the_edges_of_the_work_area() {
        let m = magnets(&main_screen(), 110.0);
        assert_eq!(m[0], Point::new(960.0, 32.0));
        assert_eq!(m[1], Point::new(126.0, 32.0));
        assert_eq!(m[2], Point::new(1794.0, 32.0));
        assert_eq!(m[3], Point::new(960.0, 1008.0));
        assert_eq!(m[5], Point::new(1794.0, 1008.0));
        assert_eq!(m[6], Point::new(126.0, 520.0));
        assert_eq!(m[7], Point::new(1794.0, 520.0));
        assert_eq!(top_center(&main_screen(), 110.0), m[0]);
    }

    #[test]
    fn magnets_follow_the_scale_and_the_screen_origin() {
        let left = screen(-2560, 0, 2560, 1400, 1.5);
        let m = magnets(&left, 110.0);
        assert_eq!(m[0], Point::new(-1280.0, 48.0));
        assert_eq!(m[1], Point::new(-2560.0 + 189.0, 48.0));
        assert_eq!(m[3], Point::new(-1280.0, 1352.0));
    }

    #[test]
    fn a_close_magnet_catches_the_island() {
        let s = main_screen();
        assert_eq!(drop_target(&s, 110.0, Point::new(1000.0, 90.0), Point::default(), false), Point::new(960.0, 32.0));
        assert_eq!(drop_target(&s, 110.0, Point::new(200.0, 950.0), Point::default(), false), Point::new(126.0, 1008.0));
    }

    #[test]
    fn far_from_magnets_the_island_stays_where_it_lands() {
        let s = main_screen();
        assert_eq!(drop_target(&s, 110.0, Point::new(700.0, 400.0), Point::default(), false), Point::new(700.0, 400.0));
    }

    #[test]
    fn magnetic_mode_always_snaps_to_the_nearest_point() {
        let s = main_screen();
        assert_eq!(drop_target(&s, 110.0, Point::new(700.0, 400.0), Point::default(), true), Point::new(960.0, 32.0));
        assert_eq!(drop_target(&s, 110.0, Point::new(300.0, 600.0), Point::default(), true), Point::new(126.0, 520.0));
    }

    #[test]
    fn momentum_carries_the_island_and_the_screen_stops_it() {
        let s = main_screen();
        let thrown = drop_target(&s, 110.0, Point::new(700.0, 400.0), Point::new(1000.0, 500.0), false);
        assert_eq!(thrown, Point::new(820.0, 460.0));
        let far = drop_target(&s, 110.0, Point::new(700.0, 600.0), Point::new(-20_000.0, 0.0), false);
        assert_eq!(far, Point::new(126.0, 520.0), "clamped at the left edge, then caught by the left magnet");
        let wall = drop_target(&s, 110.0, Point::new(1500.0, 300.0), Point::new(10_000.0, 0.0), false);
        assert_eq!(wall, Point::new(1802.0, 300.0), "8 px from the right edge");
    }

    #[test]
    fn the_pill_never_leaves_the_work_area() {
        let s = main_screen();
        assert_eq!(clamp_center(&s, 110.0, Point::new(-50.0, -50.0)), Point::new(118.0, 28.0));
        assert_eq!(clamp_center(&s, 110.0, Point::new(5000.0, 5000.0)), Point::new(1802.0, 1012.0));
        let tiny = screen(0, 0, 200, 40, 1.0);
        assert_eq!(clamp_center(&tiny, 110.0, Point::new(0.0, 0.0)), Point::new(100.0, 20.0));
    }

    #[test]
    fn release_speed_comes_from_the_last_moves() {
        let moves = [(0.0, Point::new(0.0, 0.0)), (0.05, Point::new(30.0, 10.0)), (0.125, Point::new(100.0, 20.0))];
        assert_eq!(release_speed(&moves), Point::new(800.0, 160.0));
        assert_eq!(release_speed(&moves[..1]), Point::default());
        assert_eq!(release_speed(&[]), Point::default());
        let same = [(1.0, Point::new(0.0, 0.0)), (1.0, Point::new(16.0, 0.0))];
        assert!((release_speed(&same).x - 1000.0).abs() < 1e-9, "the demo's 16 ms");
    }

    #[test]
    fn at_the_top_center_the_island_opens_downward() {
        let f = frame(&main_screen(), Point::new(960.0, 32.0));
        assert_eq!(f.window, (470, 0));
        assert_eq!(f.anchor, Point::new(490.0, 32.0));
        assert!(f.opens_down);
    }

    #[test]
    fn in_the_bottom_half_the_island_opens_upward() {
        let f = frame(&main_screen(), Point::new(960.0, 1008.0));
        assert_eq!(f.window, (470, 320));
        assert_eq!(f.anchor, Point::new(490.0, 688.0));
        assert!(!f.opens_down);
        assert!(opens_down(&main_screen(), 519.0));
        assert!(!opens_down(&main_screen(), 520.0));
    }

    #[test]
    fn near_an_edge_the_anchor_moves_inside_the_window() {
        let corner = frame(&main_screen(), Point::new(126.0, 32.0));
        assert_eq!(corner.window, (0, 0));
        assert_eq!(corner.anchor, Point::new(126.0, 32.0));
        let right = frame(&main_screen(), Point::new(1794.0, 1008.0));
        assert_eq!(right.window, (940, 320));
        assert_eq!(right.anchor, Point::new(854.0, 688.0));
        let low = frame(&main_screen(), Point::new(960.0, 500.0));
        assert!(low.opens_down);
        assert_eq!(low.window, (470, 320), "the window stops at the bottom of the work area");
        assert_eq!(low.anchor, Point::new(490.0, 180.0));
    }

    #[test]
    fn the_frame_follows_the_scale_and_the_monitor() {
        let s = screen(1920, -200, 2560, 1400, 1.5);
        let f = frame(&s, Point::new(1920.0 + 1280.0, -200.0 + 48.0));
        assert_eq!(f.window, (1920 + 1280 - 735, -200));
        assert_eq!(f.anchor, Point::new(490.0, 32.0));
    }

    #[test]
    fn a_window_larger_than_the_screen_sticks_to_the_side_the_island_opens_from() {
        let small = screen(0, 0, 1280, 680, 1.0);
        let top = frame(&small, Point::new(640.0, 32.0));
        assert_eq!(top.window, (150, 0));
        let bottom = frame(&small, Point::new(640.0, 648.0));
        assert_eq!(bottom.window, (150, -40));
        assert_eq!(bottom.anchor, Point::new(490.0, 688.0));
        let narrow = screen(0, 0, 800, 600, 1.0);
        assert_eq!(frame(&narrow, Point::new(400.0, 32.0)).window.0, -90);
    }

    #[test]
    fn the_window_follows_the_pill_with_a_fixed_anchor() {
        assert_eq!(window_at(Point::new(1000.0, 500.0), Point::new(490.0, 32.0), 1.0), (510, 468));
        assert_eq!(window_at(Point::new(1000.0, 500.0), Point::new(490.0, 32.0), 1.25), (388, 460));
    }

    #[test]
    fn the_position_survives_a_new_resolution() {
        let before = Area { x: 0, y: 0, width: 1920, height: 1040 };
        let share = to_share(before, Point::new(480.0, 1008.0));
        assert!((share.x - 0.25).abs() < 1e-9);
        let after = Area { x: 0, y: 0, width: 2560, height: 1400 };
        let p = from_share(after, share).unwrap();
        assert!((p.x - 640.0).abs() < 1e-9 && (p.y - 1356.923).abs() < 1e-3);
        assert_eq!(from_share(after, Point::new(f64::NAN, 0.5)), None);
        assert_eq!(from_share(after, Point::new(2.0, -1.0)), Some(Point::new(2560.0, 0.0)));
    }

    #[test]
    fn the_spring_settles_on_its_target_without_a_big_overshoot() {
        let mut spring = Spring::new(0.0, 1000.0, 0.0);
        let mut highest: f64 = 0.0;
        let mut frames = 0;
        while spring.step(1.0 / 60.0) {
            highest = highest.max(spring.value);
            frames += 1;
            assert!(frames < 120, "rests within two seconds");
        }
        assert_eq!(spring.value, 1000.0);
        assert!(highest < 1020.0, "overshoot {highest}");
    }

    #[test]
    fn the_spring_keeps_the_throw() {
        let mut spring = Spring::new(0.0, 0.0, 600.0);
        spring.step(0.05);
        assert!(spring.value > 10.0, "carried forward first");
        while spring.step(1.0 / 60.0) {}
        assert_eq!(spring.value, 0.0);
    }
}
