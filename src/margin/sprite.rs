//! ASCII grids to SVG, and the run-merging that makes the atlas affordable.
//!
//! A sprite is a rectangle of characters; `.` is transparent and every other
//! character indexes a palette. Emitting one `<rect>` per opaque pixel, or even
//! one per run, was measured at roughly four times the bytes of what this does:
//! collect every horizontal run of a colour and emit them as subpaths of a
//! single `<path>` per colour. The atlas is inlined into the head of every page,
//! so that factor is the difference between the feature being free and not.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A palette maps grid characters to literal colours.
///
/// Deliberately literal, never a CSS custom property: Flexoki's tokens invert
/// between light and dark, so a sprite built from them inverts too. Matter does
/// not change colour when the lights go out — only the light on it does, and
/// that lives in the stylesheet's atmosphere layer, not here.
pub type Palette = &'static [(char, &'static str)];

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub w: u32,
    pub h: u32,
    pub rows: &'static [&'static str],
}

impl Sprite {
    /// Rows that aren't exactly `w` wide, or a row count that isn't `h`.
    /// Hand-authored grids go ragged constantly; this is how we find out at
    /// `cargo test` time rather than by looking at a broken page.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.rows.len() as u32 != self.h {
            out.push(format!("height {} but {} rows", self.h, self.rows.len()));
        }
        for (i, row) in self.rows.iter().enumerate() {
            if row.chars().count() as u32 != self.w {
                out.push(format!(
                    "row {i} is {} chars, expected {}",
                    row.chars().count(),
                    self.w
                ));
            }
        }
        out
    }

    /// Characters used by the grid that the palette has no colour for.
    pub fn unpainted(&self, palette: Palette) -> Vec<char> {
        let mut missing: Vec<char> = Vec::new();
        for row in self.rows {
            for ch in row.chars() {
                if ch != '.' && !palette.iter().any(|(c, _)| *c == ch) && !missing.contains(&ch) {
                    missing.push(ch);
                }
            }
        }
        missing
    }
}

/// Horizontal runs of one colour, keyed by grid character.
///
/// `BTreeMap` rather than `HashMap` purely so the emitted atlas is byte-stable
/// across builds — an atlas that reshuffles on every run would churn the
/// content hash of every page for no reason.
fn runs(sprite: &Sprite) -> BTreeMap<char, Vec<(u32, u32, u32)>> {
    let mut out: BTreeMap<char, Vec<(u32, u32, u32)>> = BTreeMap::new();
    for (y, row) in sprite.rows.iter().enumerate() {
        let cells: Vec<char> = row.chars().collect();
        let mut x = 0usize;
        while x < cells.len() {
            let ch = cells[x];
            if ch == '.' {
                x += 1;
                continue;
            }
            let mut n = 1usize;
            while x + n < cells.len() && cells[x + n] == ch {
                n += 1;
            }
            out.entry(ch).or_default().push((x as u32, y as u32, n as u32));
            x += n;
        }
    }
    out
}

/// One `<symbol>` holding one `<path>` per colour.
pub fn symbol(id: &str, sprite: &Sprite, palette: Palette) -> String {
    let mut svg = format!(
        r#"<symbol id="{id}" viewBox="0 0 {} {}">"#,
        sprite.w, sprite.h
    );
    for (ch, spans) in runs(sprite) {
        let Some((_, colour)) = palette.iter().find(|(c, _)| *c == ch) else {
            // Unpainted characters are dropped rather than defaulting to black:
            // a hole is easier to spot than a wrong colour, and `unpainted()`
            // has already reported it as a test failure.
            continue;
        };
        let mut d = String::new();
        for (x, y, n) in spans {
            let _ = write!(d, "M{x} {y}h{n}v1h-{n}z");
        }
        let _ = write!(svg, r#"<path fill="{colour}" d="{d}"/>"#);
    }
    svg.push_str("</symbol>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAL: Palette = &[('a', "#111111"), ('b', "#222222")];

    #[test]
    fn merges_horizontal_runs_into_one_subpath() {
        let s = Sprite { w: 4, h: 1, rows: &["aaaa"] };
        let out = symbol("t", &s, PAL);
        assert!(out.contains("M0 0h4v1h-4z"), "got {out}");
        assert_eq!(out.matches("<path").count(), 1);
    }

    #[test]
    fn one_path_per_colour_not_per_run() {
        let s = Sprite { w: 4, h: 2, rows: &["abab", "aabb"] };
        let out = symbol("t", &s, PAL);
        assert_eq!(out.matches("<path").count(), 2, "got {out}");
    }

    #[test]
    fn transparent_cells_break_runs() {
        let s = Sprite { w: 5, h: 1, rows: &["aa.aa"] };
        let out = symbol("t", &s, PAL);
        assert!(out.contains("M0 0h2v1h-2z"), "got {out}");
        assert!(out.contains("M3 0h2v1h-2z"), "got {out}");
    }

    #[test]
    fn emission_is_byte_stable() {
        let s = Sprite { w: 4, h: 2, rows: &["abab", "baba"] };
        assert_eq!(symbol("t", &s, PAL), symbol("t", &s, PAL));
    }

    #[test]
    fn defects_catch_ragged_grids() {
        let s = Sprite { w: 4, h: 2, rows: &["aaaa", "aaa"] };
        let d = s.defects();
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("row 1"), "got {d:?}");
    }

    #[test]
    fn defects_catch_wrong_row_count() {
        let s = Sprite { w: 2, h: 3, rows: &["aa", "aa"] };
        assert!(s.defects().iter().any(|d| d.contains("height 3")));
    }

    #[test]
    fn unpainted_reports_missing_palette_entries() {
        let s = Sprite { w: 3, h: 1, rows: &["abz"] };
        assert_eq!(s.unpainted(PAL), vec!['z']);
    }

    #[test]
    fn unpainted_characters_are_dropped_not_guessed() {
        let s = Sprite { w: 2, h: 1, rows: &["az"] };
        let out = symbol("t", &s, PAL);
        assert_eq!(out.matches("<path").count(), 1, "got {out}");
    }
}
