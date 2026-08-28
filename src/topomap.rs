//! Decorative topographic-contour pattern for the empty space beside the
//! home page's avatar and social icons (`.profile-decor` in `main.css`).
//!
//! Each post gets its own peak or basin — a smooth radial bump positioned
//! and shaped by a hash of that post's own slug, so the terrain grows one
//! visible feature per post rather than being globally reshuffled by every
//! edit. Landmarks are layered over a faint value-noise "ground" seeded from
//! the site's author, so an empty or single-post site still reads as terrain
//! rather than a bare circle. The summed field is walked with marching
//! squares at a few quantile thresholds to trace contour lines.
//!
//! Purely decorative: `build` returns `None` if the field ever turns out
//! degenerate (flat, no crossings), which should only happen for a
//! pathological seed.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

/// Field resolution. The aspect ratio (150:40 = 3.75) matches `.profile-decor`
/// in `main.css` so the `<svg>` can stretch to fill it via
/// `preserveAspectRatio="none"` without visibly distorting the pattern.
const SAMPLE_W: usize = 150;
const SAMPLE_H: usize = 40;

/// Coarse control-point lattice the ambient ground texture is interpolated
/// from — this is what keeps landmark bumps from reading as perfect circles.
const LATTICE_W: usize = 9;
const LATTICE_H: usize = 4;
/// How much the ambient ground contributes next to a landmark's own bump
/// (which contributes up to 1.0). Kept low: it's texture, not a feature.
const AMBIENT_WEIGHT: f32 = 0.35;

/// Contour levels, as quantiles of the field's own min/max so lines appear
/// regardless of how many landmarks are in play or how they overlap.
const QUANTILES: [f32; 5] = [0.22, 0.38, 0.54, 0.70, 0.86];

/// Deterministic pseudo-random value in `[0, 1)` for one lattice vertex.
fn lattice_value(seed: &str, x: i64, y: i64) -> f32 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    x.hash(&mut hasher);
    y.hash(&mut hasher);
    let h = hasher.finish();
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Deterministic pseudo-random value in `[0, 1)` for one `(key, tag)` pair —
/// used to derive a landmark's independent parameters (position, radius,
/// sign) from a single stable key without them correlating with each other.
fn tagged_value(key: &str, tag: &str) -> f32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    tag.hash(&mut hasher);
    let h = hasher.finish();
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smoothed bilinear interpolation of the lattice at fine-grid coordinate
/// `(fx, fy)` — classic value noise, used as the ambient ground texture.
fn field_at(seed: &str, fx: usize, fy: usize) -> f32 {
    let u = fx as f32 / (SAMPLE_W - 1) as f32 * (LATTICE_W - 1) as f32;
    let v = fy as f32 / (SAMPLE_H - 1) as f32 * (LATTICE_H - 1) as f32;
    let x0 = u.floor() as i64;
    let y0 = v.floor() as i64;
    let tx = smoothstep(u - x0 as f32);
    let ty = smoothstep(v - y0 as f32);
    let v00 = lattice_value(seed, x0, y0);
    let v10 = lattice_value(seed, x0 + 1, y0);
    let v01 = lattice_value(seed, x0, y0 + 1);
    let v11 = lattice_value(seed, x0 + 1, y0 + 1);
    let top = v00 + (v10 - v00) * tx;
    let bottom = v01 + (v11 - v01) * tx;
    top + (bottom - top) * ty
}

/// One post's terrain feature: a smooth radial bump (or dip) with compact
/// support, so it only affects the field within its own `radius`.
struct Landmark {
    cx: f32,
    cy: f32,
    radius: f32,
    sign: f32,
}

/// Derive a landmark's position, size and polarity entirely from `key`
/// (a post's slug). Independent of every other post, so adding or removing
/// one post never moves another's feature.
fn landmark_for(key: &str) -> Landmark {
    Landmark {
        cx: tagged_value(key, "x") * (SAMPLE_W - 1) as f32,
        cy: tagged_value(key, "y") * (SAMPLE_H - 1) as f32,
        radius: 10.0 + tagged_value(key, "r") * 14.0,
        sign: if tagged_value(key, "s") < 0.5 { -1.0 } else { 1.0 },
    }
}

/// Value of one landmark's bump at fine-grid coordinate `(fx, fy)`, zero
/// outside its radius. Cubic falloff (`w^3`) is zero *and* has zero slope at
/// the boundary, so overlapping bumps blend without a visible seam ring.
fn landmark_value(lm: &Landmark, fx: f32, fy: f32) -> f32 {
    let dx = fx - lm.cx;
    let dy = fy - lm.cy;
    let d2 = dx * dx + dy * dy;
    let r2 = lm.radius * lm.radius;
    if d2 >= r2 {
        return 0.0;
    }
    let w = 1.0 - d2 / r2;
    lm.sign * w * w * w
}

/// One point where a contour crosses a cell edge, interpolated by value.
fn lerp_point(thr: f32, p0: (f32, f32), v0: f32, p1: (f32, f32), v1: f32) -> (f32, f32) {
    let denom = v1 - v0;
    let t = if denom.abs() < f32::EPSILON {
        0.5
    } else {
        ((thr - v0) / denom).clamp(0.0, 1.0)
    };
    (p0.0 + (p1.0 - p0.0) * t, p0.1 + (p1.1 - p0.1) * t)
}

/// Marching squares over the whole field at one threshold, returning line
/// segments as `(x0, y0, x1, y1)`. Complementary corner cases (e.g. all-above
/// vs all-below) trace the same segments, so the table only needs 8 entries.
fn contour_segments(field: &[Vec<f32>], thr: f32) -> Vec<(f32, f32, f32, f32)> {
    let mut segs = Vec::new();
    for y in 0..SAMPLE_H - 1 {
        for x in 0..SAMPLE_W - 1 {
            let tl = field[y][x];
            let tr = field[y][x + 1];
            let br = field[y + 1][x + 1];
            let bl = field[y + 1][x];
            let case = (tl >= thr) as u8
                | ((tr >= thr) as u8) << 1
                | ((br >= thr) as u8) << 2
                | ((bl >= thr) as u8) << 3;
            let canonical = case.min(15 - case);
            if canonical == 0 {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let top = || lerp_point(thr, (xf, yf), tl, (xf + 1.0, yf), tr);
            let right = || lerp_point(thr, (xf + 1.0, yf), tr, (xf + 1.0, yf + 1.0), br);
            let bottom = || lerp_point(thr, (xf, yf + 1.0), bl, (xf + 1.0, yf + 1.0), br);
            let left = || lerp_point(thr, (xf, yf), tl, (xf, yf + 1.0), bl);
            match canonical {
                1 => {
                    let (a, b) = (top(), left());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                2 => {
                    let (a, b) = (top(), right());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                3 => {
                    let (a, b) = (left(), right());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                4 => {
                    let (a, b) = (right(), bottom());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                5 => {
                    // Saddle case: resolve consistently as two segments
                    // rather than picking a per-cell orientation.
                    let (a, b) = (top(), left());
                    segs.push((a.0, a.1, b.0, b.1));
                    let (a, b) = (right(), bottom());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                6 => {
                    let (a, b) = (top(), bottom());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                7 => {
                    let (a, b) = (left(), bottom());
                    segs.push((a.0, a.1, b.0, b.1));
                }
                _ => unreachable!("canonical case is folded into 0..=7"),
            }
        }
    }
    segs
}

/// Bit pattern of a point, used as a `HashMap` key. Segment endpoints shared
/// by adjacent cells are bit-for-bit identical (same threshold, same shared
/// corner values feeding the same `lerp_point` call), so exact float-bit
/// equality is sufficient to detect a shared point — no epsilon needed.
type PointKey = (u32, u32);

fn point_key(p: (f32, f32)) -> PointKey {
    (p.0.to_bits(), p.1.to_bits())
}

fn other_endpoint(seg: (f32, f32, f32, f32), p: (f32, f32)) -> (f32, f32) {
    let a = (seg.0, seg.1);
    if point_key(a) == point_key(p) {
        (seg.2, seg.3)
    } else {
        a
    }
}

fn unused_neighbors(
    adjacency: &HashMap<PointKey, Vec<usize>>,
    used: &[bool],
    p: (f32, f32),
) -> Vec<usize> {
    adjacency
        .get(&point_key(p))
        .into_iter()
        .flatten()
        .copied()
        .filter(|&i| !used[i])
        .collect()
}

/// Follow unambiguous connections (exactly one unused segment at the current
/// point) onward from `start`, marking each segment used as it's consumed.
/// Stops at a true dead end (no unused segment left) or a junction (more
/// than one), which happens where a saddle case or an adjacent contour makes
/// the next step ambiguous — safe to stop there, it just yields a shorter
/// chain rather than wrong geometry.
fn walk_chain(
    segs: &[(f32, f32, f32, f32)],
    adjacency: &HashMap<PointKey, Vec<usize>>,
    used: &mut [bool],
    start: (f32, f32),
) -> Vec<(f32, f32)> {
    let mut chain = vec![start];
    let mut current = start;
    loop {
        let candidates = unused_neighbors(adjacency, used, current);
        if candidates.len() != 1 {
            break;
        }
        let idx = candidates[0];
        used[idx] = true;
        let next = other_endpoint(segs[idx], current);
        chain.push(next);
        current = next;
    }
    chain
}

/// Merge connected marching-squares segments into polylines, so the SVG
/// emits one `M` plus several `L`s per contour strand instead of a
/// disconnected `M`+`L` pair per grid-cell crossing. Order of the returned
/// chains (and of points within a chain) depends only on `segs`' own order,
/// so output stays deterministic despite using a `HashMap` internally.
fn chain_segments(segs: &[(f32, f32, f32, f32)]) -> Vec<Vec<(f32, f32)>> {
    let mut adjacency: HashMap<PointKey, Vec<usize>> = HashMap::new();
    for (i, &(x0, y0, x1, y1)) in segs.iter().enumerate() {
        adjacency.entry(point_key((x0, y0))).or_default().push(i);
        adjacency.entry(point_key((x1, y1))).or_default().push(i);
    }

    let mut used = vec![false; segs.len()];
    let mut chains = Vec::new();

    // Pass 1: open chains — start at any point that's a true dead end
    // (exactly one unused segment touches it), and walk both ways from it.
    for i in 0..segs.len() {
        if used[i] {
            continue;
        }
        let (x0, y0, x1, y1) = segs[i];
        for start in [(x0, y0), (x1, y1)] {
            if used[i] {
                break;
            }
            if unused_neighbors(&adjacency, &used, start).len() == 1 {
                used[i] = true;
                let other = other_endpoint(segs[i], start);
                let mut chain = vec![start];
                chain.extend(walk_chain(segs, &adjacency, &mut used, other));
                chains.push(chain);
            }
        }
    }

    // Pass 2: whatever's left is closed loops (every point degree 2) or
    // junction-bounded fragments — walk each remaining segment as far as it
    // goes in one direction.
    for i in 0..segs.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let (x0, y0, x1, y1) = segs[i];
        let mut chain = vec![(x0, y0)];
        chain.extend(walk_chain(segs, &adjacency, &mut used, (x1, y1)));
        chains.push(chain);
    }

    chains
}

/// Render a self-contained `<svg>` (no `width`/`height`, sized entirely by
/// its container) tracing one contour terrain: one landmark per entry in
/// `post_slugs`, order-independent, over an ambient ground texture seeded
/// from `ambient_seed` (the site's author). Falls back to a single landmark
/// keyed on `ambient_seed` when there are no posts yet, so a fresh site
/// still shows a feature. Returns `None` if the field turns out flat, which
/// should only happen for a pathological seed.
pub fn build(ambient_seed: &str, post_slugs: &[&str]) -> Option<String> {
    let landmarks: Vec<Landmark> = if post_slugs.is_empty() {
        vec![landmark_for(ambient_seed)]
    } else {
        post_slugs.iter().map(|slug| landmark_for(slug)).collect()
    };

    let mut field = vec![vec![0.0f32; SAMPLE_W]; SAMPLE_H];
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for (y, row) in field.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let (xf, yf) = (x as f32, y as f32);
            let mut v = AMBIENT_WEIGHT * field_at(ambient_seed, x, y);
            for lm in &landmarks {
                v += landmark_value(lm, xf, yf);
            }
            *cell = v;
            min = min.min(v);
            max = max.max(v);
        }
    }
    if max - min < f32::EPSILON {
        return None;
    }

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" preserveAspectRatio="none" fill="none" stroke="currentColor">"#,
        SAMPLE_W - 1,
        SAMPLE_H - 1
    );
    let mut any_segment = false;
    for (i, q) in QUANTILES.iter().enumerate() {
        let thr = min + (max - min) * q;
        let segs = contour_segments(&field, thr);
        if segs.is_empty() {
            continue;
        }
        any_segment = true;
        // Calibrated against --stone, not --tx-3: this range (~1.3-1.8:1
        // effective contrast) reads as a consistent, subtle elevation
        // gradient in both light and dark themes — see the color comment
        // on `.profile-decor svg` in main.css for why the token changed.
        let opacity = 0.16 + 0.045 * i as f32;
        // One decimal place is already ~0.3px of positioning error at this
        // canvas's typical display size — well below anti-aliasing's own
        // fuzziness, let alone the 0.4-unit (~1.3px) stroke width. Chaining
        // connected segments into single polylines removes the redundant
        // `M` command and duplicate coordinate at every shared endpoint.
        let mut d = String::new();
        for chain in chain_segments(&segs) {
            let mut points = chain.into_iter();
            if let Some((x0, y0)) = points.next() {
                let _ = write!(d, "M{x0:.1} {y0:.1}");
                for (x, y) in points {
                    let _ = write!(d, "L{x:.1} {y:.1}");
                }
            }
        }
        let _ = write!(
            svg,
            r#"<path d="{d}" stroke-width="0.4" opacity="{opacity:.2}" vector-effect="non-scaling-stroke"/>"#
        );
    }
    svg.push_str("</svg>");

    if any_segment {
        Some(svg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_posts_are_deterministic() {
        let a = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        let b = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn landmark_order_does_not_matter() {
        let a = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        let b = build("Ada Lovelace", &["writing-a-post", "hello-rustoki"]).unwrap();
        assert_eq!(
            a, b,
            "each post's landmark is keyed on its own slug, so order must not matter"
        );
    }

    #[test]
    fn adding_a_post_changes_the_pattern() {
        let a = build("Ada Lovelace", &["hello-rustoki"]).unwrap();
        let b = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_authors_produce_different_patterns() {
        let a = build("Ada Lovelace", &["hello-rustoki"]).unwrap();
        let b = build("Grace Hopper", &["hello-rustoki"]).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn output_is_a_self_contained_svg() {
        let svg = build("Some Author", &["a-post"]).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        let opening_tag = &svg[..svg.find('>').unwrap()];
        assert!(!opening_tag.contains("width=\""));
        assert!(!opening_tag.contains("height=\""));
    }

    #[test]
    fn no_posts_still_produces_a_pattern() {
        // A fresh site with no posts yet should still show a feature rather
        // than an empty container.
        assert!(build("Some Author", &[]).is_some());
    }

    #[test]
    fn chaining_is_deterministic_and_lossless() {
        let a = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        let b = build("Ada Lovelace", &["hello-rustoki", "writing-a-post"]).unwrap();
        assert_eq!(a, b, "chaining must not introduce nondeterminism");
        // Every `M` starts a chain; there should be strictly fewer of them
        // than the un-chained segment count would produce, proving chaining
        // actually merged something rather than emitting one path per edge.
        assert!(a.matches('M').count() < a.matches('L').count());
    }

    #[test]
    fn chain_segments_preserves_every_endpoint() {
        // A simple open zigzag plus a disconnected segment: chaining should
        // merge the connected run into one polyline and leave the isolated
        // segment as its own, without dropping or duplicating any endpoint.
        let segs = vec![
            (0.0, 0.0, 1.0, 1.0),
            (1.0, 1.0, 2.0, 0.0),
            (5.0, 5.0, 6.0, 6.0),
        ];
        let chains = chain_segments(&segs);
        let total_points: usize = chains.iter().map(|c| c.len()).sum();
        // 3 points for the merged zigzag + 2 for the isolated segment.
        assert_eq!(total_points, 5);
        assert!(chains.iter().any(|c| c.len() == 3));
        assert!(chains.iter().any(|c| c.len() == 2));
    }
}
