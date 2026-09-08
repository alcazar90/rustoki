---
title: "A Garden Made of Posts"
date: 2026-08-28
slug: a-garden-made-of-posts
description: "How the home page's raked-sand decoration works: a constant bed, one stone per recent post, and grooves that circle each stone."
tags: ["demo", "rustoki", "generative"]
---

Look to the right of the avatar on this site's home page. That faint field
of hairlines isn't a static image — it's a raked-sand garden generated at
build time, and the stones set into it are this site's newest posts. This
one, in fact, just became a stone by virtue of existing.\footnote{Which is a
little vertiginous to write, since the post describing the garden is also
sitting in it.}

## Karesansui

A dry landscape garden is a bed of fine gravel with a few stones in it. The
gravel is raked into long parallel grooves, and around each stone the rake
is walked in circles, so the grooves bend outward, ring the stone, and
close in again on the far side. Ryōan-ji in Kyoto has fifteen stones on a
bed the size of a tennis court; the rest is empty, and the emptiness is the
point \citep{vantonder2002}. The decoration follows the same rule: the bed
is constant, the stones are few, and everything else is left alone.

## The bed

The grooves are horizontal lines six units apart, inset half a gap from the
top and bottom edges so the bed doesn't begin with a line on its border.
They don't depend on any content — an empty site is a freshly raked bed
with nothing set in it yet.

## One stone per recent post

Only the three newest posts get a stone. A garden with a handful of stones
reads as a garden; one with forty reads as gravel. Each stone is a pebble
whose size follows the post's reading time, in three coarse steps: a note,
an article, an essay. Coarse on purpose, since at hairline scale a
continuous size would mean nothing.

The pebble is placed by hashing the post's own slug, and nothing else. This
is what keeps the garden stable: a stone never moves once set, and a new
post only ever retires the oldest one. But two stones cannot share ground,
so placement is a small search:

```rust
(0..PLACEMENT_ATTEMPTS)
    .map(|attempt| Stone {
        cx: x_lo + (x_hi - x_lo) * unit(seed.key, "x", attempt),
        cy: y_lo + (y_hi - y_lo) * unit(seed.key, "y", attempt),
        ..template
    })
    .find(|candidate| !placed.iter().any(|s| overlaps(s, candidate)))
```

Each attempt hashes the slug together with the attempt number, so the
sequence of candidates is fixed per post, and the first one that doesn't
cross an already-placed stone wins. This is rejection sampling, the naive
cousin of the dart-throwing that Poisson-disk methods make fast
\citep{bridson2007}; with at most three stones the fast version would be
overkill, and the naive one has a property the fast one lacks — the first
stone's position depends on its own slug alone.\footnote{A stone that finds
no free spot in its candidate sequence is simply left out. The build never
fails over decoration.}

## Raking around a stone

This is the part that makes it a garden rather than a chart. A stone's
influence extends a fixed reach $R$ beyond its inner radius $r$, the
circle just clear of the pebble. Consider a groove whose rest height is
$\delta$ above or below the stone's centre, with $0 \le \delta < R$. It is
assigned its own circle around the stone, its radius interpolated between
the inner circle and the reach:

$$
\rho(\delta) = r + \delta \, \frac{R - r}{R} \label{radius}
$$

and at horizontal offset $x$ from the stone's centre it runs at whichever is
further out — its own line, or that circle:

$$
y(x) = \max\!\left(\delta,\ \sqrt{\rho(\delta)^2 - x^2}\right) \label{arc}
$$

```rust
fn deflect(&self, dx: f32, dy: f32) -> f32 {
    let reach = self.reach();
    let inner = self.inner();
    let offset = dy.abs();
    if offset >= reach {
        return dy;
    }
    let radius = inner + offset * (reach - inner) / reach;
    let arc = (radius * radius - dx * dx).max(0.0).sqrt();
    let side = if dy < 0.0 { -1.0 } else { 1.0 };
    side * offset.max(arc)
}
```

The groove nearest the centre gets $\rho = r$ in \eqref{radius} and traces
a semicircle hugging the pebble. The groove at the reach gets $\rho = R$,
which is where it already runs, so \eqref{arc} leaves it straight. Every
groove between follows a slightly larger circle than the one before, which
is exactly what the rake leaves: concentric rings that each meet the
straight strokes where the arc comes back down to the line.\footnote{The
meeting is a corner, not a tangent. That is deliberate — in a real garden
the rake's circles butt against the straight strokes, they don't blend into
them.} The one groove that would run straight through the centre parts
around it: a rest height of exactly zero goes over the stone, and the next
one down goes under.

Because the stones' reaches are never allowed to cross, each groove is bent
by at most one stone at any $x$, and the bed can be emitted as one SVG path:
straight `H` runs where nothing reaches, short polylines where something
does.

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
background — and it is hard to argue with the name. Swapping the token, and
lowering the per-level opacity to match, fixed light mode without any
theme-specific CSS at all.

## Why deterministic, not random

The tempting version of this feature reseeds on every build, so the garden
is always freshly raked. I didn't build that one: a static site generator's
contract is that the same content produces the same output, and a homepage
that changes on every deploy with zero content changes makes `public/`
diffs meaningless. A garden shaped by *content* — one stone per recent
post, keyed on something as stable as a slug — gets the visual variety
without breaking that contract. The monks rake every morning; the build
rakes once per post.
