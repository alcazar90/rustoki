//! Decorative raked-sand garden for the empty space beside the home page's
//! profile row (`.profile-decor` in `main.css`).
//!
//! A karesansui: a bed of fine parallel grooves, as a rake leaves them, with
//! a few stones set into it. Around each stone the grooves bend outward and
//! close in again on the far side, the way the rake is walked around a rock,
//! so the nearest ones hug the stone. The stones are the site's newest posts, at most
//! [`MAX_STONES`] of them: each is placed by a hash of its own slug, so a
//! stone never moves once set and a new post only ever retires the oldest
//! one, and each is sized by its reading time in a few coarse steps. The
//! groove field itself is constant, so an empty site still shows a raked bed.
//!
//! Purely decorative: a stone that cannot be placed without crossing another
//! is simply left out, and nothing here can fail the build.

use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

/// Canvas size in SVG user units. The `<svg>` is drawn at this size and
/// anchored to the right of its box (`preserveAspectRatio="xMaxYMid meet"`),
/// so unlike a stretched image it never distorts; the box in `main.css` is
/// the content column (700px less 2rem of padding either side) by 128px, so
/// at full width the two coincide exactly.
pub const WIDTH: f32 = 636.0;
pub const HEIGHT: f32 = 128.0;

/// Distance between neighbouring grooves: one rake width.
const GROOVE_GAP: f32 = 6.0;
/// How far, in grooves, a stone's influence reaches: grooves this many
/// rake widths out bend around it, further ones run straight.
const BEND_GROOVES: f32 = 2.5;
/// Bare sand between a pebble's edge and the groove hugging it.
const PADDING: f32 = 2.0;
/// Two stones' bends may touch but never cross: a groove bent by two
/// stones at once would jump between them.
const STONE_SPACING: f32 = 0.0;
/// Stones are set into the right-hand part of the bed, so the garden
/// gathers where the band is fully visible and thins out toward the text.
const STONE_X_MIN: f32 = 404.0;
/// Inside the shortest groove end any stone row can have, so a bend is
/// never cut off by the bed's edge.
const STONE_X_MAX: f32 = 604.0;
/// Sampling step, in user units, along a bent stretch of groove.
const BEND_STEP: f32 = 2.5;
/// How much shorter the top and bottom grooves are than the middle one at
/// either end, so the bed is a rounded patch rather than a rectangle: the
/// sweep of an arm, not the edge of a frame.
const SHOULDER: f32 = 90.0;
/// Bare canvas past the longest groove's right end, so the patch floats in
/// the column instead of being clipped by it.
const END_MARGIN: f32 = 8.0;
/// Each groove's ends are nudged by up to this much either way, so no two
/// stop in line and the rounded edge is ragged rather than drawn.
const END_JITTER: f32 = 7.0;
/// How many hash-derived candidate positions a stone gets before it is
/// left out rather than overlap a neighbour. Three stones fill the field
/// fairly tightly, so the search needs room: at this many attempts a few
/// triplets in a hundred lose their third stone, which is the acceptable
/// cost of never crossing bends.
const PLACEMENT_ATTEMPTS: u32 = 256;
/// Newest posts that get a stone. A real garden has a handful, however
/// large the bed; more than this and the bends crowd into noise.
pub const MAX_STONES: usize = 3;

/// One post's claim to a stone.
#[derive(Debug, Clone, Copy)]
pub struct Seed<'a> {
    /// Stable identity, normally the post's slug.
    pub key: &'a str,
    /// Reading time in minutes; sets the stone's size.
    pub reading_minutes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Stone {
    cx: f32,
    cy: f32,
    /// Semi-axes of the pebble itself.
    rx: f32,
    ry: f32,
    /// Rotation of the pebble's long axis, degrees.
    angle: f32,
}

impl Stone {
    /// Radius of the circle the nearest groove traces around the pebble.
    fn inner(&self) -> f32 {
        self.rx.max(self.ry) + PADDING
    }

    /// Radius past which the straight grooves are undisturbed.
    fn reach(&self) -> f32 {
        self.inner() + GROOVE_GAP * BEND_GROOVES
    }

    /// Where a groove that would run `dy` above or below the stone's centre
    /// actually runs, `dx` along from it. Each groove within reach is given
    /// its own circle about the stone, the nearest one just clear of the
    /// pebble and the outermost as wide as the reach, and follows that
    /// circle wherever it lies further out than the groove's own line: a
    /// rake walked round the rock, meeting the straight strokes where the
    /// arc comes back down to them. Past the reach this is the identity.
    fn deflect(&self, dx: f32, dy: f32) -> f32 {
        let reach = self.reach();
        let inner = self.inner();
        let offset = dy.abs();
        if offset >= reach {
            return dy;
        }
        let radius = inner + offset * (reach - inner) / reach;
        let arc = (radius * radius - dx * dx).max(0.0).sqrt();
        // A groove dead on the centre line goes above the stone; the one a
        // rake width below it goes below, so the two part around it.
        let side = if dy < 0.0 { -1.0 } else { 1.0 };
        side * offset.max(arc)
    }
}

/// Uniform in `[0, 1)` from a key and a tag naming which parameter this is,
/// so one slug yields independent values for position, size and angle.
fn unit(key: &str, tag: &str, attempt: u32) -> f32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    tag.hash(&mut hasher);
    attempt.hash(&mut hasher);
    (hasher.finish() % 10_007) as f32 / 10_007.0
}

/// Pebble size from reading time, in three steps: a note, an article, an
/// essay. Coarse on purpose, so the size means something at hairline scale.
fn pebble_radius(reading_minutes: u32) -> f32 {
    match reading_minutes {
        0..=3 => 6.0,
        4..=9 => 8.5,
        _ => 11.0,
    }
}

fn overlaps(a: &Stone, b: &Stone) -> bool {
    let dx = a.cx - b.cx;
    let dy = a.cy - b.cy;
    let min = a.reach() + b.reach() + STONE_SPACING;
    dx * dx + dy * dy < min * min
}

/// Place a stone for `seed` clear of `placed`, or `None` if none of its
/// candidate positions is free.
fn place(seed: Seed<'_>, placed: &[Stone]) -> Option<Stone> {
    let r = pebble_radius(seed.reading_minutes);
    let rx = r * 1.25;
    let ry = r * 0.85;
    let angle = -25.0 + 50.0 * unit(seed.key, "angle", 0);
    let template = Stone {
        cx: 0.0,
        cy: 0.0,
        rx,
        ry,
        angle,
    };
    // Keep the whole bend inside the canvas: an arc cut off by the edge
    // reads as a mistake rather than a stone near the edge.
    let reach = template.reach();
    let (x_lo, x_hi) = (STONE_X_MIN + reach, STONE_X_MAX - reach);
    let (y_lo, y_hi) = (reach, HEIGHT - reach);
    if x_hi <= x_lo || y_hi <= y_lo {
        return None;
    }
    (0..PLACEMENT_ATTEMPTS)
        .map(|attempt| Stone {
            cx: x_lo + (x_hi - x_lo) * unit(seed.key, "x", attempt),
            cy: y_lo + (y_hi - y_lo) * unit(seed.key, "y", attempt),
            ..template
        })
        .find(|candidate| !placed.iter().any(|s| overlaps(s, candidate)))
}

/// Closed ellipse path, optionally rotated about its centre.
fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32, angle: f32) -> String {
    let (sin, cos) = angle.to_radians().sin_cos();
    let (dx, dy) = (rx * cos, rx * sin);
    let (x0, y0) = (cx - dx, cy - dy);
    let (x1, y1) = (cx + dx, cy + dy);
    format!("M{x0:.1} {y0:.1}A{rx:.1} {ry:.1} {angle:.1} 1 0 {x1:.1} {y1:.1}A{rx:.1} {ry:.1} {angle:.1} 1 0 {x0:.1} {y0:.1}Z")
}

/// Path data for one groove at rest height `y` running from `x0` to `x1`:
/// straight `H` runs where no stone reaches, sampled polylines where one
/// does.
fn groove_path(y: f32, (x0, x1): (f32, f32), stones: &[Stone], d: &mut String) {
    // Stretches of x where some stone bends this groove, left to right.
    let mut bends: Vec<(f32, f32, &Stone)> = stones
        .iter()
        .filter(|s| (y - s.cy).abs() < s.reach())
        .map(|s| (s.cx - s.reach(), s.cx + s.reach(), s))
        .collect();
    bends.sort_by(|a, b| a.0.total_cmp(&b.0));

    let _ = write!(d, "M{x0:.1} {y:.1}");
    let mut x = x0;
    for (lo, hi, stone) in bends {
        let (lo, hi) = (lo.max(x0), hi.min(x1));
        if hi <= lo {
            continue;
        }
        if lo > x {
            let _ = write!(d, "H{lo:.1}");
        }
        let mut px = lo;
        while px < hi {
            let py = stone.cy + stone.deflect(px - stone.cx, y - stone.cy);
            let _ = write!(d, "L{px:.1} {py:.1}");
            px += BEND_STEP;
        }
        x = hi;
        let _ = write!(d, "L{x:.1} {y:.1}");
    }
    if x < x1 {
        let _ = write!(d, "H{x1:.1}");
    }
}

/// Each groove's rest height and extent. Heights are evenly spaced, inset
/// from both edges by half a gap so the bed doesn't start with a line on
/// the canvas edge. Extents follow a rounded envelope: the middle groove
/// runs nearly the full width, the outermost ones fall a shoulder short at
/// both ends, and every end is jittered by a hash of its row, so the patch
/// has the ragged, rounded outline of a raked area rather than a frame.
/// None of it depends on content: the bed is the same for every site.
fn groove_rows() -> impl Iterator<Item = (f32, (f32, f32))> {
    let count = (HEIGHT / GROOVE_GAP) as usize;
    (0..count).map(move |i| {
        let y = GROOVE_GAP * (i as f32 + 0.5);
        let v = (y - HEIGHT / 2.0) / (HEIGHT / 2.0);
        let shortfall = SHOULDER * (1.0 - (1.0 - v * v).sqrt());
        let jitter = |tag: &str| END_JITTER * (2.0 * unit("bed", tag, i as u32) - 1.0);
        let x0 = (shortfall + jitter("start")).max(0.0);
        let x1 = (WIDTH - END_MARGIN - shortfall + jitter("end")).min(WIDTH - 1.0);
        (y, (x0, x1))
    })
}

/// Render the garden for `seeds`, newest first. Only the first
/// [`MAX_STONES`] get a stone; the rest are the reason the bed is raked.
pub fn build(seeds: &[Seed<'_>]) -> String {
    let mut stones: Vec<Stone> = Vec::new();
    for seed in seeds.iter().take(MAX_STONES) {
        if let Some(stone) = place(*seed, &stones) {
            stones.push(stone);
        }
    }

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="xMaxYMid meet" fill="none" stroke="currentColor" stroke-width="0.5" vector-effect="non-scaling-stroke">"#
    );

    // The bed: every groove in one path, bends and all, faintest level.
    let mut d = String::new();
    for (y, extent) in groove_rows() {
        groove_path(y, extent, &stones, &mut d);
    }
    // Opacity is left to CSS (`--i` selects a per-theme base/step in
    // `.profile-decor svg`) rather than baked in here, since the same
    // stroke alpha reads at very different effective contrast against
    // light vs dark `--bg` — see the comment there.
    let _ = write!(svg, r#"<path d="{d}" style="--i:0"/>"#);

    // The stones themselves: outlined and lightly filled, the crispest
    // things in the bed.
    let mut d = String::new();
    for s in &stones {
        d.push_str(&ellipse_path(s.cx, s.cy, s.rx, s.ry, s.angle));
    }
    if !d.is_empty() {
        let _ = write!(
            svg,
            r#"<path d="{d}" style="--i:4" fill="currentColor" fill-opacity="0.35"/>"#
        );
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeds<'a>(keys: &[(&'a str, u32)]) -> Vec<Seed<'a>> {
        keys.iter()
            .map(|(key, m)| Seed {
                key,
                reading_minutes: *m,
            })
            .collect()
    }

    fn stones_for(keys: &[(&str, u32)]) -> Vec<Stone> {
        let mut placed = Vec::new();
        for seed in seeds(keys).iter().take(MAX_STONES) {
            if let Some(s) = place(*seed, &placed) {
                placed.push(s);
            }
        }
        placed
    }

    #[test]
    fn same_posts_are_deterministic() {
        let a = build(&seeds(&[("hello", 3), ("second", 7)]));
        let b = build(&seeds(&[("hello", 3), ("second", 7)]));
        assert_eq!(a, b);
    }

    #[test]
    fn a_stone_never_moves_when_a_newer_post_arrives() {
        // Placement reads only the stone's own slug, so an older stone
        // keeps its spot as long as it is still among the newest few.
        let before = stones_for(&[("hello", 3)]);
        let after = stones_for(&[("newer", 5), ("hello", 3)]);
        assert!(after.contains(&before[0]), "{before:?} vs {after:?}");
    }

    #[test]
    fn only_the_newest_posts_get_stones() {
        let many: Vec<(&str, u32)> = (0..12)
            .map(|i| {
                (
                    ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"][i],
                    5,
                )
            })
            .collect();
        assert!(stones_for(&many).len() <= MAX_STONES);
        let svg = build(&seeds(&many));
        assert!(svg.matches("--i:4").count() == 1, "one stone layer: {svg}");
    }

    #[test]
    fn stones_never_overlap() {
        for combo in [
            [("one", 1), ("two", 12), ("three", 6)],
            [("alpha", 12), ("beta", 12), ("gamma", 12)],
            [("x", 2), ("y", 2), ("z", 2)],
        ] {
            let stones = stones_for(&combo);
            for (i, a) in stones.iter().enumerate() {
                for b in &stones[i + 1..] {
                    assert!(!overlaps(a, b), "{a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn stones_and_their_bends_stay_inside_the_canvas() {
        let stones = stones_for(&[("one", 20), ("two", 20), ("three", 20)]);
        for s in &stones {
            let r = s.reach();
            assert!(s.cx - r >= STONE_X_MIN && s.cx + r <= WIDTH, "{s:?}");
            assert!(s.cy - r >= 0.0 && s.cy + r <= HEIGHT, "{s:?}");
        }
    }

    #[test]
    fn grooves_bend_around_every_stone_and_run_straight_elsewhere() {
        let stones = stones_for(&[("one", 1), ("two", 12)]);
        assert!(!stones.is_empty());
        for s in &stones {
            // A groove aimed at the stone's centre clears the pebble.
            let above = s.deflect(0.0, 0.0);
            assert!(above >= s.inner(), "{above} inside {s:?}");
            // One a rake width below parts the other way.
            assert!(s.deflect(0.0, -GROOVE_GAP) <= -s.inner());
            // Beyond the reach nothing moves.
            assert_eq!(s.deflect(s.reach(), 1.0), 1.0);
            assert_eq!(s.deflect(0.0, s.reach()), s.reach());
            // And the bend is continuous at the reach boundary.
            let edge = s.deflect(s.reach() - 0.01, s.reach() - 0.01);
            assert!((edge - (s.reach() - 0.01)).abs() < 0.5, "{edge}");
            // The arc never dips below the groove's own line.
            for dx in [-20.0, -5.0, 0.0, 5.0, 20.0] {
                assert!(s.deflect(dx, 9.0) >= 9.0);
            }
        }
        // A groove clear of every stone is one straight line.
        let clear = stones
            .iter()
            .fold(0.0_f32, |y, s| y.max(s.cy + s.reach() + 1.0));
        if clear < HEIGHT {
            let mut d = String::new();
            groove_path(clear, (0.0, WIDTH), &stones, &mut d);
            assert_eq!(d, format!("M0.0 {clear:.1}H{WIDTH:.1}"));
        }
    }

    #[test]
    fn the_bed_is_a_rounded_patch_not_a_rectangle() {
        let rows: Vec<(f32, (f32, f32))> = groove_rows().collect();
        let (top, middle, bottom) = (rows[0].1, rows[rows.len() / 2].1, rows[rows.len() - 1].1);
        // Outer grooves are markedly shorter than the middle one, at both ends.
        assert!(top.0 > middle.0 + SHOULDER / 2.0, "{top:?} vs {middle:?}");
        assert!(top.1 < middle.1 - SHOULDER / 2.0, "{top:?} vs {middle:?}");
        assert!(
            bottom.1 < middle.1 - SHOULDER / 2.0,
            "{bottom:?} vs {middle:?}"
        );
        // No groove reaches the column edge, and none is degenerate.
        for (_, (x0, x1)) in &rows {
            assert!(*x0 >= 0.0 && *x1 <= WIDTH - END_MARGIN + END_JITTER);
            assert!(x1 - x0 > WIDTH / 2.0);
        }
        // The ends are ragged: not every neighbouring pair steps the same way.
        let steps: Vec<f32> = rows.windows(2).map(|w| w[1].1 .1 - w[0].1 .1).collect();
        assert!(steps.iter().any(|s| *s > 0.0) && steps.iter().any(|s| *s < 0.0));
        // And every stone row's bed still holds the widest possible bend.
        let widest = Stone {
            cx: 0.0,
            cy: 0.0,
            rx: pebble_radius(u32::MAX) * 1.25,
            ry: 0.0,
            angle: 0.0,
        }
        .reach();
        for (y, (_, x1)) in &rows {
            if (*y - HEIGHT / 2.0).abs() <= HEIGHT / 2.0 - widest {
                assert!(
                    *x1 >= STONE_X_MAX,
                    "row {y} ends at {x1} inside the stone field"
                );
            }
        }
    }

    #[test]
    fn longer_posts_get_larger_stones() {
        assert!(pebble_radius(1) < pebble_radius(5));
        assert!(pebble_radius(5) < pebble_radius(30));
    }

    #[test]
    fn no_posts_still_produces_a_raked_bed() {
        let svg = build(&[]);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("--i:0"), "missing grooves: {svg}");
        assert!(!svg.contains("--i:4"), "stone with no post: {svg}");
    }

    #[test]
    fn output_is_a_self_contained_svg() {
        let svg = build(&seeds(&[("hello", 3)]));
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains(r#"preserveAspectRatio="xMaxYMid meet""#));
        assert!(!svg.contains("NaN"));
    }
}
