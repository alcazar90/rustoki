---
title: "Hello, Rustoky"
date: 2024-01-15
description: "A short tour of what rustoky renders out of the box: code, math, and citations."
tags: ["demo", "rustoky"]
---

Rustoky turns Markdown into static HTML, with a few things handled for you
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

## Headings and the table of contents

Any post with two or more headings — like this one — gets an automatic table
of contents, built from the headings above.
