//! Choreography. A routine is a list of beats; the runtime plays beats and
//! knows nothing else, so adding a new one is adding a `Routine` const here and
//! listing it on a character. No JavaScript changes.
//!
//! Coordinates are in sprite pixels, measured from the stage's left edge and
//! floor, so changing `[margin] scale` moves everything together and nothing
//! has to be re-tuned.

/// Stage height, in sprite pixels. The road he walks up.
pub const ROAD: i32 = 110;

/// Fully below the stage floor: the crossing starts and ends off-frame.
const OFF: i32 = -27;
/// Beside the prop.
const AT: i32 = 15;
/// Far enough up that the dissolve reads as distance, not as a switch.
const MID: i32 = 57;
/// Clear of the top edge.
const OUT: i32 = 113;

pub struct Beat {
    pub name: &'static str,
    pub ms: u32,
    /// Feet above the stage floor.
    pub y: i32,
    pub opacity: f32,
    /// Runs the two-frame walk cycle. Overrides `pose`.
    pub walking: bool,
    /// Frame role to hold while standing still.
    pub pose: &'static str,
    /// Whether the prop is lit. Only visible in the dark, where the prop sits
    /// near-invisible until his lantern reaches it.
    pub prop_lit: bool,
    /// How long the opacity change takes. Separate from `ms` so a dissolve can
    /// fade across its whole beat while every other beat snaps.
    pub fade_ms: u32,
}

pub struct Routine {
    pub name: &'static str,
    pub beats: &'static [Beat],
    /// Relative weight when picking one crossing out of the character's set.
    pub weight: u32,
}

pub fn total_ms(r: &Routine) -> u32 {
    r.beats.iter().map(|b| b.ms).sum()
}

const fn beat(
    name: &'static str,
    ms: u32,
    y: i32,
    opacity: f32,
    walking: bool,
    pose: &'static str,
    prop_lit: bool,
    fade_ms: u32,
) -> Beat {
    Beat { name, ms, y, opacity, walking, pose, prop_lit, fade_ms }
}

/// The default crossing. He walks up the page away from the reader, stops once
/// at the prop, turns, and does nothing for three and a half seconds — that
/// empty beat is the whole feature — then goes on and dissolves rather than
/// exiting, because walking off an edge is staging and dissolving is memory.
pub const LOOK_BACK: Routine = Routine {
    name: "look_back",
    weight: 5,
    beats: &[
        beat("approach", 7000, AT, 1.0, true, "walk_a", false, 900),
        beat("arrive", 600, AT, 1.0, false, "walk_a", true, 900),
        beat("turn", 500, AT, 1.0, false, "face", true, 900),
        beat("look back", 3500, AT, 1.0, false, "face", true, 900),
        beat("turn away", 500, AT, 1.0, false, "walk_a", true, 900),
        beat("leave", 3500, MID, 1.0, true, "walk_a", false, 900),
        beat("dissolve", 3500, OUT, 0.0, true, "walk_a", false, 3400),
        beat("absence", 5000, OUT, 0.0, false, "walk_a", false, 200),
    ],
};

/// The rare one. He sits down instead. A thing that happens every time is a
/// widget; a thing that happens sometimes is a memory — so this is weighted to
/// turn up about once in six crossings and is not worth optimising for.
pub const THE_REST: Routine = Routine {
    name: "the_rest",
    weight: 1,
    beats: &[
        beat("approach", 7000, AT, 1.0, true, "walk_a", false, 900),
        beat("arrive", 600, AT, 1.0, false, "walk_a", true, 900),
        beat("sit down", 9000, AT, 1.0, false, "rest", true, 900),
        beat("stand up", 600, AT, 1.0, false, "walk_a", true, 900),
        beat("leave", 3500, MID, 1.0, true, "walk_a", false, 900),
        beat("dissolve", 3500, OUT, 0.0, true, "walk_a", false, 3400),
        beat("absence", 5000, OUT, 0.0, false, "walk_a", false, 200),
    ],
};

/// Where a crossing starts and ends, for the runtime's reset.
pub const START_Y: i32 = OFF;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_routine_starts_and_ends_off_stage() {
        for r in [&LOOK_BACK, &THE_REST] {
            let last = r.beats.last().unwrap();
            assert!(
                last.opacity == 0.0,
                "{} ends visible; the vanish is the point",
                r.name
            );
            assert!(last.y > ROAD, "{} ends on stage at y={}", r.name, last.y);
        }
    }

    #[test]
    fn every_routine_pauses_at_least_three_seconds() {
        // The pause is the content. A routine that never stops is a widget.
        for r in [&LOOK_BACK, &THE_REST] {
            let held: u32 = r.beats.iter().filter(|b| !b.walking).map(|b| b.ms).sum();
            assert!(held >= 3000, "{} only holds for {held}ms", r.name);
        }
    }

    #[test]
    fn crossings_are_slow_enough_to_read_as_presence() {
        for r in [&LOOK_BACK, &THE_REST] {
            assert!(
                total_ms(r) >= 20_000,
                "{} crosses in {}ms; anything quicker reads as a widget",
                r.name,
                total_ms(r)
            );
        }
    }

    #[test]
    fn every_beat_is_named() {
        // Names are how a dropped routine reports which beat wanted a frame
        // the character does not have.
        for r in [&LOOK_BACK, &THE_REST] {
            for b in r.beats {
                assert!(!b.name.is_empty(), "unnamed beat in {}", r.name);
            }
        }
    }
}
