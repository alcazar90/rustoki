# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rustoki` — a standalone Rust static site generator (not a Cargo workspace,
one binary crate) that reads Markdown content and renders it to static HTML.
It was extracted from a personal blog's repo so the generator can be
installed and reused independently of any one site's content. It ships with
one baked-in Flexoki-themed look — see "Design principles" below for what
that means for extending it.

A site that uses `rustoki` supplies a `content/` directory (config, posts,
pages, static assets) and runs the installed binary against it; this repo
owns no content of its own.

## Commands

```sh
# Build the site: content/ -> public/ (run from a site's root directory)
cargo run --release -- build

# Build including draft: true posts, for local preview only
cargo run --release -- build --drafts

# Build (optionally --drafts), then serve public/ at http://127.0.0.1:8000/ (--port <n> to override)
cargo run --release -- serve --drafts

# Scaffold a new post at content/posts/YYYY-MM-DD-<slug>.md
cargo run --release -- new-post "My Post Title"

# Run all tests
cargo test

# Run tests for one module
cargo test render::   # filter by path prefix, e.g. render:: or templates::

# Install the binary from a local checkout, to use against a site elsewhere
cargo install --path . --locked
```

There is no lint/format CI step defined in this repo beyond `cargo test`; use `cargo fmt`/`cargo clippy` as normal Rust hygiene if touching code.

## Layout

- `src/` — all production logic (single binary crate, `[[bin]] name = "rustoki"`).
- `templates/*.html` — minijinja templates, baked into the binary via `include_str!` (not read from disk at runtime).
- `styles/main.css` — single stylesheet, inlined into every page's `<head>` at build time via `include_str!` (no external CSS request, no separate CSS file ships in `public/`). `styles/margin.css` is a second sheet appended to it only when a site opts into the margin figure.
- `example/` — a small demo site (its own `content/`) used to preview the generator and produce the screenshots in `README.md`. Not part of the crate; run against it with `cargo run --manifest-path ../Cargo.toml -- serve` from inside `example/`.

At runtime (i.e. in whatever site directory the binary is run from), it expects:
- `content/config.toml` (site config), `content/posts/*.md`, `content/pages/*.md`, `content/static/` (copied verbatim to `public/`).
- `public/` — build output. Deleted and regenerated on every `build`.

## Build pipeline (`src/`)

`main.rs` orchestrates the whole build:

1. Load `content/config.toml` (`config.rs`) — flat schema, one struct, no nesting. `avatar` is an optional path/URL to a profile image on the home page; leaving it unset omits the image and lays out the social-links row without the space reserved for it (see `index.html`'s `.no-avatar` branch). Includes optional `[giscus]` block for comments (present/absent toggles whether post pages render a giscus mount).
2. Walk `content/posts/` and `content/pages/` (`content.rs`) — parses both `---` YAML and `+++` TOML frontmatter leniently (real-world content tends to have inconsistent shapes like `slug: []` or `tags: "single"`), skips drafts, sorts by date descending. Files under `pages/` become `SourceKind::Page`, everything else is a `Post`.
3. Render each source through the Markdown pipeline (`render/mod.rs`), which walks `pulldown-cmark`'s event stream and intercepts:
   - **Headings** → auto `id=` anchors + collected into a Table of Contents (rendered inline as a `<nav class="toc">` block if a post has ≥2 headings).
   - **Fenced code blocks** → `render/code.rs`, syntect-based syntax highlighting emitting CSS classes (not inline colors), so theming can change without a rebuild. Unknown languages fall back to a plain unhighlighted block — highlighting must never fail the build.
   - **Inline/display math** → `render/math.rs`, LaTeX → MathML via `pulldown-latex`. Also does a source-level preprocessing pass: normalizes legacy MathJax delimiters (`\(...\)`, `\[...\]`) to `$`/`$$`, resolves `\label`/`\ref`/`\eqref` into sequential equation numbers, wraps standalone `\begin{equation}` blocks, and fixes `\\` line breaks in display math (pulldown-latex rejects top-level `\\` outside an environment — see `fix_display_math_newlines`) and bare `<br>` spacer lines that CommonMark would otherwise swallow as an HTML block (`isolate_bare_br_tags`). Malformed math falls back to an escaped `<code>` block rather than panicking. Each `\label`'d display-math block gets an `id=` anchor, so `\ref`/`\eqref` render as links (`<a href="#key">N</a>`) that jump to the defining equation. Sections have no equation-style label/counter — cross-reference a heading with a plain markdown link to its auto-generated id, not `\ref`.
   - **Figures** → `render/figure.rs`. `<figure>`/`<figcaption>` are CommonMark "type 6" HTML blocks, so pulldown-cmark treats them as opaque raw HTML and skips inline parsing (including math) on their contents; this module renders `$...$`/`$$...$$` inside figures before the main parse. It also mirrors the equation label system with an independent figure counter: every `<figure id="...">` is auto-numbered in document order, `\figref{key}` resolves to a linked "Figure N", and each `<figcaption>` gets a "Figure N. " prefix inserted.
   - **Footnotes → sidenotes** (`render/footnote.rs`) — two author syntaxes, one rendering. A LaTeX `\footnote{…}` is rewritten into CommonMark footnote syntax (`[^fn-N]` at the reference, `[^fn-N]: …` appended as a definition) *before* the parser runs, so from there on it is indistinguishable from a hand-written `[^key]` footnote. Brace matching is a scan, not a regex — real footnote bodies contain `\frac{a}{b}` and other nested groups — and `\footnote` inside a code span or fence is left literal. Definitions are intercepted in `transform_events` and their bodies fed back through `transform_events` itself, so math, citations, links and code inside a note render by exactly the rules the prose does, with no second renderer to keep in sync. The reference site emits an HTML-comment placeholder (pulldown-cmark reaches a definition only *after* every reference to it); `footnote::splice` fills them in once the document is walked. Presentation: a numbered mark plus a note the stylesheet floats into the right gutter above 1150px and collapses into a checkbox-toggled inline block below it — no script. A note is *phrasing* content, so `<p>`/`<div>` in a body are rewritten to spans (`phrasing`); a `<pre>` code block inside a note is unsupported. A reference with no definition degrades to a bare number.
   - **Bibliography** — if a `<post-stem>.refs.yaml` sidecar exists next to a post (see `render/bibliography.rs`), `\cite{key}`/`\citep{key}` become numbered superscript links and a `<section class="references">` is appended.
   - **Embedded tweets** (`render/tweet.rs`) — `<blockquote class="twitter-tweet">` blocks (X's standard embed snippet) are replaced with a static, pre-themed `.tweet-card` div built from a build-time snapshot of the tweet, rather than upgraded client-side by X's `widgets.js`. The snapshot is fetched from X's public (unofficial) syndication endpoint (`cdn.syndication.twimg.com/tweet-result`, the same one `widgets.js` calls, requiring a `token` derived from the tweet id — see `syndication_token`) and cached at `content/tweet-cache.json`, keyed by tweet id; a site should commit that cache so rebuilds are fully offline once a tweet has been fetched once. If a tweet isn't cached and the fetch fails (offline build, deleted tweet, endpoint changes), the original `<blockquote>` markup passes through untouched rather than failing the build. The card reuses the site's own Flexoki CSS variables, so it matches light/dark instantly with the rest of the page — no client-side script, no re-theming flash.
   - **Relative image paths** → rewritten to `/posts/<slug>/<filename>`; absolute/schemed/root-relative URLs pass through untouched.
   - **Images** (`render/assets.rs`, applied as a final pass over the rendered HTML) — every `<img>` is repointed at a build-time WebP derivative, gets intrinsic `width`/`height` (unless the author set an explicit display size), `decoding="async"`, `loading="lazy"` on all but the first image on the page (the likely LCP element), and is wrapped in `<a class="img-original">` linking to the untouched original. Runs over the finished HTML rather than the event stream so raw-HTML `<img>` tags — inside `<figure>` blocks or written inline in legacy posts — get the same treatment as markdown images.
4. Render through minijinja templates (`templates.rs`) — one context struct per page kind (`PostContext`, `PageContext`, `IndexContext`, `Render404Context`), each wrapping a shared `RenderEnv` (site config + inlined CSS + build year).
5. Emit `public/index.html` (post listing), `public/posts/<slug>/index.html`, `public/<slug>/index.html` (pages), `public/feed.xml` (Atom, hand-rolled in `feed.rs`), `public/sitemap.xml` (also `feed.rs`), and `public/404.html`.
6. Optionally build the margin figure (`margin/mod.rs`) — see below. Its stylesheet is appended to the inlined CSS and its atlas/stage/runtime are handed to every page kind through `RenderEnv`.
7. Copy `content/static/` verbatim into `public/`, then a site's `.image-cache/` on top of it if present — originals ship alongside their derivatives so the full-resolution links resolve and no previously-published URL breaks.

Every build fully deletes and recreates `public/` — there's no incremental build.

## The margin figure (`src/margin/`)

Opt-in via a `[margin]` block in `content/config.toml`; absent, the site ships
none of it. A small pixel character crosses the page's side gutter every few
minutes, stops once at a prop, and dissolves.

Three registries drive it and nothing else does:

- `cast.rs` — `CAST`, one `Character` per entry. A character is a palette, a
  set of frames addressed by **role** (`walk_a`/`walk_b` mandatory, `face` and
  `rest` optional), an optional light-source position, and an optional `Prop`.
- `routine.rs` — choreography as a list of `Beat`s naming those roles.
  Coordinates are in **sprite pixels** from the stage's left edge and floor, so
  changing `[margin] scale` moves everything together and nothing needs
  re-tuning.
- `runtime.js` — a character-agnostic player. It plays whatever beat table the
  build emitted and knows nothing about who is walking.

So adding a character or a choreography is adding a const. No template, no
stylesheet, no script change. `sprite.rs` turns ASCII grids into SVG, merging
horizontal runs into one `<path>` per colour — one `<rect>` per run was
measured at ~4x the bytes, which matters because the atlas is inlined per page.

Two invariants worth not breaking:

- **Sprite fills are literal colours, never Flexoki tokens.** Tokens invert
  between themes, so a token-built sprite inverts with them (an early cave
  interior was a hole in light mode and a glowing white blob in dark). Only the
  atmosphere layer in `margin.css` is allowed to know which theme it is in, and
  it gets exactly one element per theme — a raking shadow by day, a lantern
  pool by night — each load-bearing rather than decorative.
- **Nothing here fails the build.** An unknown character name, a routine whose
  poses the character can't strike, a ragged grid, a palette gap: each warns on
  stderr and drops the routine or the whole feature, matching how the rest of
  the generator treats content-level problems. `cargo test` catches ragged
  grids and palette gaps for every cast member before they ever ship.

The figure never appears on narrow viewports — at `body { max-width: 700px }`
there is no gutter — and the runtime checks `innerWidth` once per crossing
rather than holding a resize listener. The bytes still ship on mobile; that is
the acknowledged cost of inlining.

## Content conventions (what a consuming site's `content/` should look like)

- Post files: `content/posts/YYYY-MM-DD-<slug>.md`. The slug is derived from the filename (date prefix stripped) unless overridden by a `slug:` frontmatter field.
- Frontmatter (YAML `---` or TOML `+++`) supports: `title`, `date`, `slug`, `tags`, `description`, `draft`, `lang`. `draft: true` posts are skipped by a plain `build`; pass `--drafts` to render them locally (with a "Draft" badge on the post page and index listing) while still excluding them from `feed.xml`/`sitemap.xml`.
- Footnotes: write `\footnote{...}` (LaTeX, for manuscript imports) or `[^key]` + `[^key]: ...` (CommonMark); both render as sidenotes and neither needs configuration. Keep the mark at a sentence boundary — below the 1150px breakpoint the note opens as a block at exactly that point, so a mid-clause mark splits the sentence around it. Display math and code blocks cannot go inside a note.
- Bibliography sidecar: `content/posts/<post-stem>.refs.yaml`, keyed by citation key, each entry has `author`, `title`, `year` required, `url`/`journal`/`booktitle`/`note` optional. **Never hand-type a `## References` section in a post's Markdown** — `bibliography::strip_references_section` unconditionally deletes a `## References` heading and everything after it from *every* post body at build time (sidecar or not), so hand-written content there is silently discarded. To add references: create the `.refs.yaml` sidecar and mark citation points inline with `\cite{key}`/`\citep{key}`; the build generates the numbered `<section class="references">` itself.
- Tweet cache: `content/tweet-cache.json`, keyed by tweet id (see `render/tweet.rs`). A site should commit it. Delete an entry to force a re-fetch on the next build.
- Images: drop full-resolution exports into `content/static/img/` and reference them normally — sizing is the build's job, not the author's. Derivatives are cached in `.image-cache/files/` (mirrors the `content/static/` tree), keyed on a content fingerprint of each source recorded in `.image-cache/sources.json`, and pruned when a source is deleted. WebP does not always win — already-quantized GIFs, flat PNGs and small icons encode *larger* than their source — so a derivative that isn't smaller is deleted and the page keeps the original. Because `public/` is a verbatim copy of the derivatives tree, keeping a rejected file around would ship it as dead weight; the verdict is recorded as `useful: false` in `sources.json` so the next build doesn't re-encode it just to reach the same answer. Freshness is deliberately content-based, not mtime-based: `git checkout` rewrites source mtimes and CI caches restore derivatives with their original ones, so an mtime check would re-encode everything on every CI run. A clean checkout re-encodes everything on the first build (~35s); warm builds are ~1s. Requires `cwebp`/`gif2webp` on PATH (`brew install webp`, `apt-get install -y webp`); without them the build still succeeds but ships images unoptimized — a site's CI should verify they're present if that matters.
- `new-post` scaffolds files with a `draft: true` frontmatter stub; it refuses to overwrite an existing file at the target path.

## Design principles evident in the code (follow these when extending)

- **Never crash the build on content-level problems.** Bad math, unknown syntax-highlighting languages, malformed frontmatter shapes, unfetchable tweets, and missing image encoders all degrade gracefully (fallback rendering, warning to stderr, or skipping the file) rather than aborting the build. Note the one place this needs a counterweight: a silently-skipped image optimization is invisible in the output, so a consuming site's CI should check for the encoders explicitly rather than trusting the graceful path.
- **Gutter features are desktop-only, and derive their own breakpoint.** The content column is centred, so a usable gutter on one side only exists once the viewport affords the same space on the other — hence the `2.0 *` in the margin figure's `min_width` (`margin/mod.rs`) and the 1150px sidenote breakpoint (`700px + 2 × (12rem + 2rem)`). The difference between the two: the figure is decoration and simply vanishes below its threshold; a sidenote is content, so it degrades to a reachable inline note instead. Sidenote CSS is written mobile-first for the same reason — the float is layered on inside the media query, never the other way round, so the worst case of any failure is a note that reads inline rather than a page with horizontal scroll.
- **Templates and CSS are compiled into the binary** (`include_str!`), not read from disk at runtime — there's no template hot-reload story, and no per-site theming. This is a deliberate scope decision (see the README): if you want a different look, fork.
- Frontmatter parsing is deliberately lenient (see `de_string_lenient`/`de_string_list_lenient` in `content.rs`) — content migrated from other generators tends to have inconsistent field shapes.
