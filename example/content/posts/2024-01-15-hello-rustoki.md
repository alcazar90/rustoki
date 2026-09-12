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

## Algorithms

A fenced block tagged `algorithm` is set as a numbered pseudocode listing,
the way LaTeX's `algorithmic` package would. One statement per line, indented
to nest; keywords such as `for … do`, `if … then`, `end for` and `return` are
picked out on their own, and every line is ordinary inline Markdown, so math
goes in `$…$` exactly as it does in the prose. Later text can point back at
it with `\algref{alg:reinforce}`, which renders as \algref{alg:reinforce}.

```algorithm
title: Vanilla policy gradient, aka REINFORCE
label: alg:reinforce
---
Input: a parameterised policy $\pi_\theta$, learning rate $\alpha$
Output: the trained policy $\pi_\theta$
for iteration $= 0, 1, 2, \dots, N$ do
    Collect a set of trajectories $\mathcal{D}^{\pi_\theta} = \{\tau^{(i)}\}$ by sampling from the current policy $\pi_\theta$
    Calculate the return $R(\tau)$ for each trajectory $\tau \in \mathcal{D}^{\pi_\theta}$
    $\hat{g} \leftarrow \frac{1}{|\mathcal{D}^{\pi_\theta}|} \sum_{\tau \in \mathcal{D}^{\pi_\theta}} \sum_{t=0}^{T-1} \nabla_\theta \log \pi_\theta(a_t \mid s_t) \, R(\tau)$ // policy gradient estimate
    $\theta \leftarrow \theta + \alpha \hat{g}$
end for
return $\pi_\theta$
```

A `function` line sets its name in small caps, and `Input:`/`Output:` lines
carry no line number, just as `\Require`/`\Ensure` do not:

```algorithm
title: Binary search
---
function BinarySearch($A[1..n]$, $x$)
    $\ell \leftarrow 1$, $r \leftarrow n$
    while $\ell \le r$ do
        $m \leftarrow \lfloor (\ell + r) / 2 \rfloor$
        if $A[m] = x$ then
            return $m$
        else if $A[m] < x$ then
            $\ell \leftarrow m + 1$
        else
            $r \leftarrow m - 1$
        end if
    end while
    return not found
end function
```

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
