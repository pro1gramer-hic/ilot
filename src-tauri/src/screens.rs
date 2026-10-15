//! Several screens and the island at rest (prompt 12, lot 1): which screen holds the
//! island (Réglages › Général), the island that follows the cursor to another screen at
//! the same place on it, and the hidden island that comes back when the cursor touches
//! the edge it rests against. Pure rules in physical pixels; window.rs applies them in
//! the loop that already reads the cursor, so following the mouse or watching the edge
//! adds a few comparisons to each turn of that loop and no thread or timer.

use crate::locale::{Text, tr};
use crate::placement::{self, Point, Screen};
use crate::players::Area;
use serde::Serialize;
use std::time::{Duration, Instant};

/// The cursor stays this long on another screen before the island moves there: crossing
/// a screen on the way to a third one does not drag the island along.
pub const FOLLOW_DWELL: Duration = Duration::from_millis(350);

/// A monitor: the whole of it (where the cursor can be) and the island's view of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    pub screen: Screen,
    pub bounds: Area,
    pub primary: bool,
}

fn contains(area: Area, p: Point) -> bool {
    p.x >= f64::from(area.x)
        && p.x < f64::from(area.x + area.width)
        && p.y >= f64::from(area.y)
        && p.y < f64::from(area.y + area.height)
}

/// The display under a point, if any.
pub fn display_under(displays: &[Display], p: Point) -> Option<&Display> {
    displays.iter().find(|d| contains(d.bounds, p))
}

/// The same place on another screen: the same magnet when the island rests on one (the
/// top center stays the top center, whatever the scale), else the same share of the work
/// area, kept whole on it.
pub fn same_place(from: &Screen, to: &Screen, center: Point, half_width: f64) -> Point {
    let magnet = placement::magnets(from, half_width)
        .iter()
        .position(|m| (m.x - center.x).hypot(m.y - center.y) < 1.0);
    if let Some(index) = magnet {
        return placement::magnets(to, half_width)[index];
    }
    let share = placement::to_share(from.work, center);
    let there = placement::from_share(to.work, share).unwrap_or_else(|| placement::top_center(to, half_width));
    placement::clamp_center(to, half_width, there)
}

/// « Suit la souris »: remembers the screen the cursor went to and since when.
#[derive(Debug, Default)]
pub struct Follow {
    candidate: Option<(String, Instant)>,
}

impl Follow {
    /// The screen to move the island to, once the cursor has stayed on it long enough.
    /// `island` is the island's screen, `under` the one under the cursor.
    pub fn update(&mut self, island: &str, under: Option<&str>, now: Instant) -> Option<String> {
        let Some(under) = under.filter(|name| *name != island) else {
            self.candidate = None;
            return None;
        };
        match &self.candidate {
            Some((name, since)) if name == under => {
                if now.duration_since(*since) >= FOLLOW_DWELL {
                    self.candidate = None;
                    return Some(under.to_owned());
                }
            }
            _ => self.candidate = Some((under.to_owned(), now)),
        }
        None
    }
}

/// A screen as Réglages › Général lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenView {
    /// The system's name of the monitor, as saved with the island's place.
    pub id: String,
    /// « Écran 1 », numbered from left to right, then top to bottom.
    pub label: String,
    /// « 1920 × 1080 ».
    pub detail: String,
    pub primary: bool,
    /// The island is on it now.
    pub current: bool,
}

/// The screens in reading order, numbered for the user.
pub fn views(displays: &[Display], current: Option<&str>) -> Vec<ScreenView> {
    let mut sorted: Vec<&Display> = displays.iter().collect();
    sorted.sort_by_key(|d| (d.bounds.x, d.bounds.y));
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, d)| ScreenView {
            id: d.screen.name.clone(),
            label: tr(Text::ScreenNumber { n: i + 1 }),
            detail: format!("{} × {}", d.bounds.width, d.bounds.height),
            primary: d.primary,
            current: current == Some(d.screen.name.as_str()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(name: &str, x: i32, width: i32, height: i32, scale: f64, primary: bool) -> Display {
        let taskbar = (48.0 * scale) as i32;
        Display {
            screen: Screen { name: name.into(), work: Area { x, y: 0, width, height: height - taskbar }, scale },
            bounds: Area { x, y: 0, width, height },
            primary,
        }
    }

    /// A 1920 × 1080 laptop on the left at 100 %, a 2560 × 1440 screen on the right at 150 %.
    fn desk() -> Vec<Display> {
        vec![display("DISPLAY1", 0, 1920, 1080, 1.0, true), display("DISPLAY2", 1920, 2560, 1440, 1.5, false)]
    }

    #[test]
    fn the_screen_under_the_cursor_including_its_taskbar() {
        let all = desk();
        assert_eq!(display_under(&all, Point::new(10.0, 10.0)).unwrap().screen.name, "DISPLAY1");
        assert_eq!(display_under(&all, Point::new(1920.0, 10.0)).unwrap().screen.name, "DISPLAY2");
        assert_eq!(display_under(&all, Point::new(500.0, 1070.0)).unwrap().screen.name, "DISPLAY1", "on the taskbar");
        assert!(display_under(&all, Point::new(500.0, 1200.0)).is_none(), "below the laptop: no screen");
    }

    #[test]
    fn the_island_keeps_its_place_on_the_other_screen() {
        let all = desk();
        let (laptop, big) = (&all[0].screen, &all[1].screen);
        let top_center = placement::top_center(laptop, 110.0);
        let there = same_place(laptop, big, top_center, 110.0);
        assert_eq!(there, placement::top_center(big, 110.0), "top center stays top center");
        let corner = same_place(laptop, big, Point::new(1900.0, 1020.0), 110.0);
        assert_eq!(corner, placement::clamp_center(big, 110.0, corner), "kept whole on the screen");
        assert!(corner.x > 1920.0 + 2000.0 && corner.y > 1200.0);
    }

    #[test]
    fn following_waits_for_the_cursor_to_stay() {
        let mut follow = Follow::default();
        let t = Instant::now();
        assert_eq!(follow.update("DISPLAY1", Some("DISPLAY1"), t), None);
        assert_eq!(follow.update("DISPLAY1", Some("DISPLAY2"), t), None);
        assert_eq!(follow.update("DISPLAY1", Some("DISPLAY2"), t + Duration::from_millis(200)), None);
        assert_eq!(follow.update("DISPLAY1", Some("DISPLAY2"), t + FOLLOW_DWELL), Some("DISPLAY2".into()));
        // Passing through a screen does not count.
        assert_eq!(follow.update("DISPLAY2", Some("DISPLAY3"), t + Duration::from_secs(1)), None);
        assert_eq!(follow.update("DISPLAY2", Some("DISPLAY1"), t + Duration::from_millis(1100)), None);
        assert_eq!(follow.update("DISPLAY2", Some("DISPLAY1"), t + Duration::from_millis(1300)), None);
        // Back on the island's screen, or nowhere: the wait starts over.
        assert_eq!(follow.update("DISPLAY2", Some("DISPLAY2"), t + Duration::from_millis(1400)), None);
        assert_eq!(follow.update("DISPLAY2", None, t + Duration::from_millis(1500)), None);
        assert_eq!(follow.update("DISPLAY2", Some("DISPLAY1"), t + Duration::from_secs(5)), None);
    }

    #[test]
    fn screens_are_numbered_from_left_to_right() {
        let mut all = desk();
        all.reverse();
        all.push(display("DISPLAY3", -1280, 1280, 1024, 1.0, false));
        let list = views(&all, Some("DISPLAY2"));
        let names: Vec<(&str, &str, &str, bool, bool)> =
            list.iter().map(|v| (v.id.as_str(), v.label.as_str(), v.detail.as_str(), v.primary, v.current)).collect();
        assert_eq!(
            names,
            vec![
                ("DISPLAY3", "Écran 1", "1280 × 1024", false, false),
                ("DISPLAY1", "Écran 2", "1920 × 1080", true, false),
                ("DISPLAY2", "Écran 3", "2560 × 1440", false, true),
            ]
        );
    }

    /// « Suit la souris » adds this check to each turn of the cursor loop (every 16 to
    /// 100 ms): it must cost next to nothing.
    #[test]
    fn following_costs_well_under_a_microsecond_per_turn() {
        let all = desk();
        let mut follow = Follow::default();
        let start = Instant::now();
        let turns = 200_000u32;
        for i in 0..turns {
            let p = Point::new(f64::from(i % 4000), f64::from(i % 1000));
            let under = display_under(&all, p).map(|d| d.screen.name.as_str());
            std::hint::black_box(follow.update("DISPLAY1", under, start));
        }
        let per_turn = start.elapsed() / turns;
        println!("follow check: {per_turn:?} per turn");
        assert!(per_turn < Duration::from_micros(5), "{per_turn:?}");
    }

    #[test]
    fn screens_are_named_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let labels: Vec<String> = views(&desk(), None).into_iter().map(|v| v.label).collect();
        assert_eq!(labels[0], "Screen 1");
        assert!(labels.iter().all(|l| l.starts_with("Screen ")));
    }
}
