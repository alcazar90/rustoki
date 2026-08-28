---
title: "A Map Made of Posts"
date: 2026-08-28
slug: a-map-made-of-posts
description: "How the home page's contour decoration works: value noise, one landmark per post, and marching squares."
tags: ["demo", "rustoki", "generative"]
---

Look beside the avatar on this site's home page. That faint contour pattern
isn't a static image — it's generated at build time, and it changes shape as
posts are added. This one, in fact, just grew a third landmark by virtue of
existing.\footnote{Which is a little vertiginous to write, since the post
describing the map is also reshaping it.}

## Three ingredients

The pattern is the sum of two fields, walked by one algorithm:

1. An **ambient ground** — smooth value noise, seeded once from the site's
   author, present everywhere as faint texture.
2. One **landmark** per post — a bump or basin positioned, sized, and signed
   by a hash of that post's own slug, independent of every other post.
3. **Marching squares**, tracing contour lines through the summed field at a
   handful of threshold levels.

## The ambient ground: value noise

A coarse lattice of control points gets an independent pseudo-random value
$v_{i,j} \in [0, 1)$ at each vertex, derived from hashing the seed together
with $(i, j)$. To sample the field at a finer point between four such
vertices, the four corner values are blended with bilinear interpolation —
but using a fractional coordinate eased through a smoothstep curve rather
than the raw fraction $t$:

$$
S(t) = 3t^2 - 2t^3
$$

Plain linear interpolation between lattice cells produces visible creases at
every cell boundary; $S(t)$ has zero slope at $t=0$ and $t=1$, so adjacent
cells meet without a seam.[^smoothstep] This is why value noise reads as
rolling terrain rather than a checkerboard.

[^smoothstep]: The same curve shows up all over interpolation and animation
    easing under the name "smoothstep" — it's just a Hermite blend between
    two flat tangents.

## One landmark per post

Each post's slug is hashed four independent times — once per parameter — to
place a landmark: an $(c_x, c_y)$ position anywhere in the canvas, a radius
$r$, and a sign $s \in \{-1, +1\}$ deciding whether it rises or sinks. At
distance $d$ from its own center, a landmark's contribution is zero once
$d \ge r$, and otherwise:

$$
w = 1 - \frac{d^2}{r^2}, \qquad b(d) = s \, w^3 \label{bump}
$$

```rust
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
```

The cube in \eqref{bump} isn't decoration: $w$ already reaches zero exactly
at the boundary, but a *quadratic* falloff still has nonzero slope there, so
two overlapping bumps would meet along a visible ring. Cubing $w$ makes both
the value *and* the slope vanish at $d = r$, so neighboring landmarks blend
into the ambient ground with no seam.\footnote{The same reasoning motivated
the smoothstep curve for the ambient ground two sections up — this whole
file is really one idea, C¹ continuity, applied twice.}

Because each landmark only reads its own slug, adding post four never moves
where posts one through three sit. The field at any point is just:

$$
\text{field}(x, y) = 0.35 \cdot \text{ground}(x, y) + \sum_{\text{post } p} b_p\!\left(\lVert (x,y) - c_p \rVert\right)
$$

## Tracing the contours: marching squares

This is the 2D relative of the classic isosurface-extraction
algorithm\footnote{Marching Cubes walks a 3D voxel grid, testing each cube's
eight corners against a threshold to look up which triangles cross it. A 2D
cell only has four corners and traces line segments instead of triangles —
otherwise it's the same idea.} \citep{lorensen1987}. For each cell of four
neighboring field samples, the four corners are each tested against a
threshold and packed into a 4-bit case number; that number indexes a lookup
table of which cell edges a contour line crosses:

```rust
let case = (tl >= thr) as u8
    | ((tr >= thr) as u8) << 1
    | ((br >= thr) as u8) << 2
    | ((bl >= thr) as u8) << 3;
let canonical = case.min(15 - case);
```

The `min(case, 15 - case)` line is doing real work: a cell where only the
top-left corner is *above* the threshold traces the same line as a cell
where only the top-left corner is *below* it — the contour doesn't care
which side is "in," only where the crossing is. Folding complementary cases
together shrinks the lookup table from sixteen entries to eight.

Five threshold levels are chosen as quantiles of the field's own min and
max — not fixed numbers — so contour lines always appear regardless of how
many landmarks are summed or how they happen to overlap.

## A color that survives both themes

The first version of this used `--tx-3`, Flexoki's faint text-gray, and it
all but vanished in light mode. The reason falls out of the WCAG contrast
formula: each channel is linearized,

$$
L = 0.2126 R + 0.7152 G + 0.0722 B,
$$

and two luminances become a contrast ratio

$$
\text{contrast} = \frac{L_1 + 0.05}{L_2 + 0.05}, \quad L_1 \ge L_2. \label{contrast}
$$

`--tx-3` against the light theme's background comes out to only $2.0$ in
\eqref{contrast} — it was tuned for dark-mode-only faint text, not this.
`--stone`, Flexoki's warm earth tone, scores $8.6$ in the same light
background — and coincidentally, "stone" is exactly the right word for a
contour map. Swapping the token, and lowering the per-band opacity to match,
fixed light mode without any theme-specific CSS at all.

## Why deterministic, not random

The tempting version of this feature reseeds on every build, so the map is
always fresh. I didn't build that one: a static site generator's contract is
that the same content produces the same output, and a homepage that changes
on every deploy with zero content changes makes `public/` diffs meaningless.
Terrain shaped by *content* — one landmark per post, keyed on something
as stable as a slug — gets the visual variety without breaking that
contract, in the same spirit as older work on modeling landscapes from
procedural, rather than hand-authored, data \citep{fournier1982}.
