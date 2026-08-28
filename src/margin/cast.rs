//! The cast. One entry per character; adding another is adding a `Character`
//! const to `CAST` and nothing else — no template change, no CSS change, no
//! runtime change. A site picks one by name in `[margin] character = "..."`.
//!
//! Frames are addressed by *role*, not by index, so a routine written for one
//! character plays on any character that has the roles it asks for. The two
//! walk frames are mandatory; `face` and `rest` are optional, and a routine
//! that needs a role its character lacks is dropped at build time with a
//! warning rather than rendering a blank sprite.

use super::routine::{Routine, LOOK_BACK, THE_REST};
use super::sprite::{Palette, Sprite};

/// Frame roles a routine may name.
pub const WALK_A: &str = "walk_a";
pub const WALK_B: &str = "walk_b";

pub struct Frame {
    pub role: &'static str,
    pub sprite: Sprite,
}

/// A thing that stands in the margin whether or not anyone is there. It is
/// what makes the margin a place rather than an animation container, so it is
/// drawn once, never animated, and outlives every crossing.
pub struct Prop {
    pub palette: Palette,
    pub sprite: Sprite,
    /// Where it stands, in sprite pixels from the stage's left edge and floor.
    pub x: u32,
    pub y: u32,
}

/// Where a character's own light source pools on the ground, in sprite pixels
/// from the figure's left edge and feet. `None` means the character carries no
/// light and the night aura is not emitted at all — the day shadow is enough
/// for a figure that reads on a dark page without help.
pub type Light = Option<(u32, u32)>;

pub struct Character {
    /// The `[margin] character` value that selects this one.
    pub name: &'static str,
    pub palette: Palette,
    pub w: u32,
    pub h: u32,
    /// Where the figure walks, in sprite pixels from the stage's left edge.
    pub x: u32,
    pub frames: &'static [Frame],
    pub light: Light,
    pub prop: Option<Prop>,
    /// Every routine this character can perform, with relative weights.
    pub routines: &'static [Routine],
}

impl Character {
    pub fn frame(&self, role: &str) -> Option<&Frame> {
        self.frames.iter().find(|f| f.role == role)
    }

    /// Total stage width in sprite pixels: the figure plus whatever the prop
    /// needs to its right.
    pub fn stage_w(&self) -> u32 {
        let fig = self.x + self.w;
        match &self.prop {
            Some(p) => fig.max(p.x + p.sprite.w),
            None => fig,
        }
    }
}

/// Every character the binary knows how to draw.
pub const CAST: &[Character] = &[TRAVELLER];

pub fn find(name: &str) -> Option<&'static Character> {
    CAST.iter().find(|c| c.name == name)
}

// ---------------------------------------------------------------------------
// The traveller. High 3/4 camera, Octopath silhouette: a wide-brim hat doing
// the identifying work, one satchel meaning he carries everything he owns, a
// coat that flares because he is moving, and a staff with a lantern on it
// because for half the hours of the day he has to be his own light source.
// ---------------------------------------------------------------------------

const TRAVELLER_PAL: Palette = &[
    ('o', "#171310"),
    ('1', "#2F4034"),
    ('2', "#47624E"),
    ('3', "#6E8F6E"),
    ('4', "#4A3A2B"),
    ('5', "#6E543A"),
    ('6', "#96784F"),
    ('7', "#7A4F2A"),
    ('8', "#A9793F"),
    ('j', "#A6743F"),
    ('k', "#C9975F"),
    ('l', "#E4BC8A"),
    ('s', "#8A7050"),
    ('r', "#C6BEA8"),
    ('g', "#6B5B35"),
    // The flame, alone in this palette, isn't a literal hex: it's the one
    // pixel meant to keep moving through a held pose, and a <path> inside
    // <symbol>/<defs> never ticks a CSS animation targeting it directly — a
    // <use> instance's rendered copy is what animates, and only an inherited
    // property (not an explicit attribute) crosses into it. `currentColor`
    // resolves against `color` on the .mg-fig element in margin.css, which is
    // what the flicker actually animates; it is still one fixed colour at
    // rest, not a theme token, so this doesn't reopen the literal-colour rule
    // sprite.rs documents — it's a mechanism, not a second appearance.
    ('f', "currentColor"),
];

const T_WALK_A: &[&str] = &[
    // left boot planted, staff down
    "..so.....oooo.....",
    "..so....o6666o....",
    "ossso...o5555o....",
    "ogoso..oo5555oo...",
    "ogfgoo6655555566o.",
    "offfoo4444444444o.",
    "ogfgo.o44444444o..",
    ".ogo...oooooooo...",
    "..so..o22222222o..",
    "..so.o2223332222o.",
    "..so.o2233332222o.",
    "..so.o222333288o..",
    "..so.o2222322288o.",
    "..kkoo22222222878o",
    "..kkoo22222222888o",
    "..so.o222222222o8o",
    "..so.o2222222222o.",
    "..so.o1112222111o.",
    "..soo111222222111o",
    "..soo111111111111o",
    "..so..o11ooo11o...",
    "..so..o11o.o11o...",
    "..so..o77o.o77o...",
    "..so..oooo........",
];

const T_WALK_B: &[&str] = &[
    // right boot planted, staff lifted one pixel
    "..so.....oooo.....",
    "ossso...o6666o....",
    "ogoso...o5555o....",
    "ogfgo..oo5555oo...",
    "offfoo6655555566o.",
    "ogfgoo4444444444o.",
    ".ogo..o44444444o..",
    "..so...oooooooo...",
    "..so..o22222222o..",
    "..so.o2223332222o.",
    "..so.o2233332222o.",
    "..so.o222333288o..",
    "..kkoo2222322288o.",
    "..kkoo22222222878o",
    "..so.o22222222888o",
    "..so.o222222222o8o",
    "..so.o2222222222o.",
    "..so.o1112222111o.",
    "..soo111222222111o",
    "..soo111111111111o",
    "..so..o11ooo11o...",
    "..so..o11o.o11o...",
    "..so..o77o.o77o...",
    "...........oooo...",
];

const T_FACE: &[&str] = &[
    // the look back
    "..so.....oooo.....",
    "..so....o6666o....",
    "ossso...o5555o....",
    "ogoso..oo5555oo...",
    "ogfgoo6655555566o.",
    "offfoo4444444444o.",
    "ogfgo.o44444444o..",
    ".ogo...oooooooo...",
    "..so..ojjjjjjjjo..",
    "..so..okokkkkoko..",
    "..so...okkllkko...",
    "..so.oo222rr222oo.",
    "..so.o2222rr2222o.",
    "..kkoo2222222222o.",
    "..kkoo2222882222o.",
    "..so.o2222222222o.",
    "..so.o2222222222o.",
    "..so.o1112222111o.",
    "..soo111222222111o",
    "..soo111111111111o",
    "..so..o11ooo11o...",
    "..so..o11o.o11o...",
    "..so..o77o.o77o...",
    "..so..oooo........",
];

const T_REST: &[&str] = &[
    // sat down, staff across the knees
    "..................",
    "..................",
    "..................",
    ".........oooo.....",
    "........o6666o....",
    "........o5555o....",
    ".......oo5555oo...",
    ".....o6655555566o.",
    ".....o4444444444o.",
    "......o44444444o..",
    ".......oooooooo...",
    "......ojjjjjjjjo..",
    "......okokkkkoko..",
    ".......okkllkko...",
    ".....oo222rr222oo.",
    ".....o2222rr2222o.",
    "....o82222rr222o..",
    "...osssssssssssso.",
    "....o22222222222o.",
    "....o11122222111o.",
    "....o11111111111o.",
    ".....o111oo111o...",
    ".....o77oo77o.....",
    ".....oooo.oooo....",
];


/// The waystone. The site's own footer mark — two stones — stood upright, so
/// the scenery comes out of the site's identity instead of out of a JRPG.
const WAYSTONE_PAL: Palette = &[
    ('o', "#171310"),
    ('a', "#4E4740"),
    ('b', "#6E655B"),
    ('c', "#8E8478"),
    ('m', "#3A342E"),
];

const WAYSTONE: &[&str] = &[
    // two stones, by the side of the road: the big one is the original
    // single-stone grid untouched, columns 7 on; the small one is tucked
    // against its base at columns 0-6, resting rather than stacked, the way
    // the footer mark's own two shapes sit side by side rather than piled.
    "...........oooo....",
    ".........oocccccoo.",
    "........occcbbbbbco",
    "........ocbbbbbbbbo",
    ".......occbbbbbbabo",
    ".......ocbbbbbbaabo",
    "..ooo..ocbbbbbaaabo",
    ".occcboocbbbbaaaabo",
    "occcbao.obbbaaaaabo",
    "obbbaao.ommbaaaaamo",
    ".omaam...ommmaaamo.",
    "..omo.....oommmoo..",
];

const TRAVELLER: Character = Character {
    name: "traveller",
    palette: TRAVELLER_PAL,
    w: 18,
    h: 24,
    x: 2,
    frames: &[
        Frame { role: WALK_A, sprite: Sprite { w: 18, h: 24, rows: T_WALK_A } },
        Frame { role: WALK_B, sprite: Sprite { w: 18, h: 24, rows: T_WALK_B } },
        Frame { role: "face", sprite: Sprite { w: 18, h: 24, rows: T_FACE } },
        Frame { role: "rest", sprite: Sprite { w: 18, h: 24, rows: T_REST } },
    ],
    // The lantern hangs off the staff on his left; the pool it throws sits on
    // the ground under it, not at the height of the flame.
    light: Some((2, 3)),
    prop: Some(Prop {
        palette: WAYSTONE_PAL,
        sprite: Sprite { w: 19, h: 12, rows: WAYSTONE },
        x: 21,
        y: 15,
    }),
    routines: &[LOOK_BACK, THE_REST],
};
