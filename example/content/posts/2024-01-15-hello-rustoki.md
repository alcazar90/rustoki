---
title: "Hello, Rustoki"
date: 2024-01-15
description: "A short tour of what rustoki renders out of the box: code, math, and citations."
tags: ["demo", "rustoki"]
---

Rustoki turns Markdown into static HTML, with a few things handled for you
along the way: syntax highlighting, LaTeX math, and a citation system. This
post is a short tour.

## Syntax highlighting

Fenced code blocks are highlighted at build time via syntect, so there's no
client-side highlighting library to ship.

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

## Math

Inline math like $E = mc^2$ renders as MathML, as does display math:

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

## Citations

Claims can cite a bibliography sidecar file inline \citep{knuth1984}, and the
build generates a numbered references section from it automatically.

## Sidenotes

Footnotes render as sidenotes. On a wide screen the note sits out in the right
margin, level with the line that refers to it; anywhere narrower, the number
becomes a toggle you can tap.\footnote{Below roughly 1150px there is no gutter
to put a note in, so it opens inline instead — as this one just did. Try
widening the window.}

Write one as LaTeX, `\footnote{...}`, so a manuscript needs no editing on its
way in — or as an ordinary markdown footnote, `[^key]`.[^markdown] Both render
identically. A note is ordinary content: it can hold math like
$\nabla_\theta \log \pi_\theta(a \mid s)$, or a citation of its
own.\footnote{As in \citep{knuth1984} — cited from inside a note, and still
numbered by where it appears in the text.} The body is walked by the same
renderer as the prose, so what works out here works in there.\footnote{Display
math and code blocks are the exception: a note is inline content, and cannot
carry a block element.}

[^markdown]: Like this one, written `[^markdown]` in the text with its
    definition on a line of its own.

## Headings and the table of contents

Any post with two or more headings — like this one — gets an automatic table
of contents, built from the headings above.
