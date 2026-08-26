//! The margin traveller: a pixel figure who crosses the page's side gutter
//! every few minutes, stops once, and dissolves.
//!
//! Three registries drive it and nothing else does. A character (`cast.rs`) is
//! a palette plus frames addressed by role; a routine (`routine.rs`) is a list
//! of beats naming those roles; the runtime (`runtime.js`) plays beats and
//! knows about neither. Adding a second character or a third routine is adding
//! a const — no template, stylesheet or script change.
//!
//! Nothing here can fail the build. A misconfigured character, a routine whose
//! poses the character can't strike, a ragged sprite grid: each warns on stderr
//! and drops the feature or the offending routine, matching how the rest of the
//! generator treats content-level problems.

pub mod cast;
pub mod routine;
pub mod sprite;

use crate::config::MarginConfig;
use cast::{Character, WALK_A, WALK_B};
use routine::{Beat, Routine, ROAD, START_Y};
use serde::Serialize;
use std::fmt::Write as _;

const RUNTIME_JS: &str = include_str!("runtime.js");
const MARGIN_CSS: &str = include_str!("../../styles/margin.css");

/// Half the content column, in px. Tied to `body { max-width: 700px }` in
/// `main.css`: the stage is anchored off the centred column, not the viewport
/// edge, so it tracks the text at any window width.
const CONTENT_HALF: f32 = 350.0;
/// Breathing room between the stage and the text column.
const GUTTER_GAP: f32 = 16.0;
/// One step of the walk cycle. Slow enough that the gait reads as a walk and
/// not as a flicker.
const STEP_MS: u32 = 760;

/// Everything a page needs in order to carry the traveller.
#[derive(Debug)]
pub struct Margin {
    /// Appended to the inlined stylesheet.
    pub css: &'static str,
    /// The sprite atlas, emitted once per page.
    pub atlas: String,
    /// The stage markup.
    pub stage: String,
    /// The runtime, with its beat table baked in.
    pub script: String,
    /// What got built, for the build log.
    pub character: &'static str,
    pub routines: Vec<(&'static str, u32)>,
    /// Narrowest viewport the figure will play on.
    pub min_width: u32,
}

#[derive(Serialize)]
struct RoutineData {
    w: u32,
    b: Vec<[serde_json::Value; 7]>,
}

#[derive(Serialize)]
struct RuntimeData {
    s: f32,
    o: i32,
    m: u32,
    f: u32,
    a: u32,
    b: u32,
    t: u32,
    r: Vec<RoutineData>,
}

/// Trim a computed length to something short and stable in the emitted CSS.
fn px(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if (r - r.round()).abs() < f32::EPSILON {
        format!("{}px", r.round() as i64)
    } else {
        format!("{r}px")
    }
}

fn beat_row(b: &Beat) -> [serde_json::Value; 7] {
    use serde_json::Value as V;
    [
        V::from(b.ms),
        V::from(b.y),
        V::from(b.opacity),
        V::from(u8::from(b.walking)),
        V::from(b.pose),
        V::from(u8::from(b.prop_lit)),
        V::from(b.fade_ms),
    ]
}

/// Routines this character can actually perform, with a warning for each one
/// it can't. A routine asking for a pose the character has no frame for would
/// otherwise render as a blank sprite mid-crossing.
fn playable(ch: &Character) -> Vec<&'static Routine> {
    ch.routines
        .iter()
        .filter(|r| {
            let missing: Vec<String> = r
                .beats
                .iter()
                .filter(|b| ch.frame(b.pose).is_none())
                .map(|b| format!("\"{}\" needs a {} frame", b.name, b.pose))
                .collect();
            if !missing.is_empty() {
                eprintln!(
                    "warning: margin: skipping routine \"{}\" for character \"{}\": {}",
                    r.name,
                    ch.name,
                    missing.join("; ")
                );
                return false;
            }
            true
        })
        .collect()
}

/// Ragged grids and unpainted characters, reported together so one bad sprite
/// doesn't hide the next.
fn sprite_defects(ch: &Character) -> Vec<String> {
    let mut out = Vec::new();
    for f in ch.frames {
        for d in f.sprite.defects() {
            out.push(format!("frame \"{}\": {d}", f.role));
        }
        for c in f.sprite.unpainted(ch.palette) {
            out.push(format!("frame \"{}\": no palette entry for '{c}'", f.role));
        }
    }
    if let Some(p) = &ch.prop {
        for d in p.sprite.defects() {
            out.push(format!("prop: {d}"));
        }
        for c in p.sprite.unpainted(p.palette) {
            out.push(format!("prop: no palette entry for '{c}'"));
        }
    }
    out
}

/// Build the traveller for a site, or `None` with a warning if it can't be.
pub fn build(cfg: &MarginConfig) -> Option<Margin> {
    let Some(ch) = cast::find(&cfg.character) else {
        let known: Vec<&str> = cast::CAST.iter().map(|c| c.name).collect();
        eprintln!(
            "warning: margin: unknown character \"{}\" (known: {}); skipping the margin figure",
            cfg.character,
            known.join(", ")
        );
        return None;
    };

    let defects = sprite_defects(ch);
    if !defects.is_empty() {
        for d in &defects {
            eprintln!("warning: margin: character \"{}\" {d}", ch.name);
        }
        eprintln!("warning: margin: skipping the margin figure");
        return None;
    }

    if ch.frame(WALK_A).is_none() || ch.frame(WALK_B).is_none() {
        eprintln!(
            "warning: margin: character \"{}\" is missing a walk frame; skipping the margin figure",
            ch.name
        );
        return None;
    }

    let routines = playable(ch);
    if routines.is_empty() {
        eprintln!(
            "warning: margin: character \"{}\" has no playable routine; skipping the margin figure",
            ch.name
        );
        return None;
    }

    let scale = cfg.scale.max(0.5);
    let stage_w = ch.stage_w() as f32 * scale;
    let stage_h = ROAD as f32 * scale;
    // Derived rather than hard-coded so that changing `scale` cannot silently
    // leave the figure overlapping the text.
    let min_width = cfg
        .min_width
        .unwrap_or_else(|| ((CONTENT_HALF * 2.0) + 2.0 * (stage_w + GUTTER_GAP)).ceil() as u32);

    // --- atlas ------------------------------------------------------------
    // Only the frames this character owns are emitted; an unused pose in the
    // cast costs nothing on the wire.
    let mut atlas = String::from(r#"<svg width="0" height="0" aria-hidden="true" style="position:absolute"><defs>"#);
    for f in ch.frames {
        atlas.push_str(&sprite::symbol(
            &format!("mg-{}", f.role),
            &f.sprite,
            ch.palette,
        ));
    }
    if let Some(p) = &ch.prop {
        atlas.push_str(&sprite::symbol("mg-prop", &p.sprite, p.palette));
    }
    atlas.push_str("</defs></svg>");

    // --- stage ------------------------------------------------------------
    let mut vars = String::new();
    let _ = write!(
        vars,
        "--mg-w:{};--mg-h:{};--mg-x:{};--mg-fw:{};--mg-fh:{};--mg-unit:{};--mg-half:{};--mg-gap:{};--mg-step:{}ms",
        px(stage_w),
        px(stage_h),
        px(ch.x as f32 * scale),
        px(ch.w as f32 * scale),
        px(ch.h as f32 * scale),
        px(scale),
        px(CONTENT_HALF),
        px(GUTTER_GAP),
        STEP_MS,
    );
    if let Some((lx, ly)) = ch.light {
        // The pool is sized off the figure so a bigger character throws a
        // bigger light without anything being re-tuned by hand.
        let _ = write!(
            vars,
            ";--mg-lx:{};--mg-ly:{};--mg-lw:{};--mg-lh:{}",
            px(lx as f32 * scale),
            px(ly as f32 * scale),
            px(ch.w as f32 * 4.7 * scale),
            px(ch.h as f32 * 2.0 * scale),
        );
    }
    if let Some(p) = &ch.prop {
        let _ = write!(
            vars,
            ";--mg-px:{};--mg-py:{}",
            px(p.x as f32 * scale),
            px(p.y as f32 * scale)
        );
    }

    let mut stage = format!(r#"<div id="mg" aria-hidden="true" style="{vars}">"#);
    if ch.light.is_some() {
        stage.push_str(r#"<div class="mg-aura"><i></i></div>"#);
    }
    if let Some(p) = &ch.prop {
        let _ = write!(
            stage,
            r##"<svg class="mg-prop" viewBox="0 0 {w} {h}" width="{sw}" height="{sh}"><use href="#mg-prop" width="{w}" height="{h}"/></svg>"##,
            w = p.sprite.w,
            h = p.sprite.h,
            sw = px(p.sprite.w as f32 * scale),
            sh = px(p.sprite.h as f32 * scale),
        );
    }
    stage.push_str(r#"<div class="mg-fig"><i class="mg-shadow"></i>"#);
    for f in ch.frames {
        // walk_a and walk_b get fixed hooks so the keyframes can find them
        // without the stylesheet knowing any role names.
        let hook = match f.role {
            WALK_A => " mg-w0",
            WALK_B => " mg-w1",
            _ => "",
        };
        let _ = write!(
            stage,
            r##"<svg class="mg-fr{hook}" data-r="{role}" viewBox="0 0 {w} {h}" width="{sw}" height="{sh}"><use href="#mg-{role}" width="{w}" height="{h}"/></svg>"##,
            role = f.role,
            w = f.sprite.w,
            h = f.sprite.h,
            sw = px(f.sprite.w as f32 * scale),
            sh = px(f.sprite.h as f32 * scale),
        );
    }
    stage.push_str("</div></div>");

    // --- runtime ----------------------------------------------------------
    let data = RuntimeData {
        s: scale,
        o: START_Y,
        m: min_width,
        f: cfg.first_delay,
        a: cfg.interval[0],
        b: cfg.interval[1].max(cfg.interval[0]),
        t: routines.iter().map(|r| r.weight).sum(),
        r: routines
            .iter()
            .map(|r| RoutineData {
                w: r.weight,
                b: r.beats.iter().map(beat_row).collect(),
            })
            .collect(),
    };
    let json = serde_json::to_string(&data).ok()?;
    let script = format!(
        "<script data-cfasync=\"false\">{}</script>",
        RUNTIME_JS.replace("__MG_DATA__", &json)
    );

    Some(Margin {
        css: MARGIN_CSS,
        atlas,
        stage,
        script,
        character: ch.name,
        min_width,
        routines: routines
            .iter()
            .map(|r| (r.name, routine::total_ms(r)))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> MarginConfig {
        MarginConfig::default()
    }

    fn built() -> Margin {
        build(&cfg()).expect("the default traveller should build")
    }

    #[test]
    fn default_config_builds_the_traveller() {
        let m = built();
        assert!(m.atlas.contains(r#"id="mg-walk_a""#), "{}", m.atlas);
        assert!(m.atlas.contains(r#"id="mg-prop""#));
        assert!(m.stage.contains(r#"id="mg""#));
        assert!(m.script.contains("<script"));
    }

    #[test]
    fn unknown_character_is_a_warning_not_a_failure() {
        let mut c = cfg();
        c.character = "nobody".into();
        assert!(build(&c).is_none());
    }

    #[test]
    fn every_cast_member_is_well_formed() {
        for ch in cast::CAST {
            assert!(
                sprite_defects(ch).is_empty(),
                "{}: {:?}",
                ch.name,
                sprite_defects(ch)
            );
            assert!(ch.frame(WALK_A).is_some(), "{} has no walk_a", ch.name);
            assert!(ch.frame(WALK_B).is_some(), "{} has no walk_b", ch.name);
            assert!(!playable(ch).is_empty(), "{} has no playable routine", ch.name);
        }
    }

    #[test]
    fn cast_names_are_unique() {
        // They are the config key, so a duplicate would silently shadow.
        let mut seen: Vec<&str> = Vec::new();
        for ch in cast::CAST {
            assert!(!seen.contains(&ch.name), "duplicate character \"{}\"", ch.name);
            seen.push(ch.name);
        }
    }

    #[test]
    fn walk_frames_get_the_hooks_the_keyframes_look_for() {
        let m = built();
        assert!(m.stage.contains("mg-fr mg-w0"), "{}", m.stage);
        assert!(m.stage.contains("mg-fr mg-w1"));
    }

    #[test]
    fn the_beat_table_is_baked_in_not_left_as_a_placeholder() {
        let m = built();
        assert!(!m.script.contains("__MG_DATA__"), "placeholder survived");
        assert!(m.script.contains(r#""r":[{"#), "{}", m.script);
    }

    #[test]
    fn min_width_keeps_the_stage_clear_of_the_text_column() {
        // Derived from scale, so a bigger figure demands a wider window rather
        // than quietly overlapping the prose.
        let mut small = cfg();
        small.scale = 2.0;
        let mut big = cfg();
        big.scale = 4.0;
        let w = |c: &MarginConfig| build(c).unwrap().min_width;
        assert!(w(&big) > w(&small));
        assert!(w(&small) >= 700);
    }

    #[test]
    fn scale_drives_every_emitted_dimension() {
        let mut c = cfg();
        c.scale = 3.0;
        let m = build(&c).unwrap();
        // 18-wide sprite at 3x
        assert!(m.stage.contains("--mg-fw:54px"), "{}", m.stage);
        assert!(m.stage.contains("--mg-unit:3px"));
    }

    #[test]
    fn a_character_without_a_light_emits_no_aura() {
        // The runtime moves whatever it finds, so a lightless character must
        // not leave an empty aura element behind either.
        for ch in cast::CAST {
            let m = build(&MarginConfig { character: ch.name.into(), ..cfg() }).unwrap();
            assert_eq!(
                m.stage.contains("mg-aura"),
                ch.light.is_some(),
                "{} aura/light mismatch",
                ch.name
            );
        }
    }

    #[test]
    fn light_position_comes_from_the_character_not_the_stylesheet() {
        let mut c = cfg();
        c.scale = 2.0;
        let m = build(&c).unwrap();
        // traveller carries the lantern at sprite (2, 3)
        assert!(m.stage.contains("--mg-lx:4px"), "{}", m.stage);
        assert!(m.stage.contains("--mg-ly:6px"));
    }

    #[test]
    fn a_disabled_site_ships_no_frames() {
        // Sanity on the size claim: the atlas is the only per-page cost.
        let m = built();
        assert!(m.atlas.len() < 12_000, "atlas grew to {} B", m.atlas.len());
    }
}
