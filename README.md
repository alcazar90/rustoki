# rustoky

A small, opinionated static site generator written in Rust. Markdown in,
static HTML out—with syntax highlighting, LaTeX math, figures, a
bibliography system, and a [Flexoki](https://github.com/kepano/flexoki)-[themed](https://stephango.com/about#colophon) default look baked in.

Built to power a single personal blog; extracted here so the generator can be
installed and reused independently of that blog's content.

![A rendered post page: syntax-highlighted code, LaTeX math, and a numbered references section, in Flexoki's dark theme](example/screenshots/post.png)

## Design

Rustoky is built around one idea: a static site should stay static. Nothing
that can be resolved at build time is deferred to the reader's browser.

- **One binary, no runtime.** `cargo install` produces a single ~6.5 MB
  executable — no Node, no npm install, nothing to provision on a CI runner
  beyond the Rust toolchain. It's built on a small set of focused crates
  (minijinja, pulldown-cmark, syntect, pulldown-latex — 16 direct
  dependencies in total), not a plugin ecosystem.
- **Rendering happens once, at build time.** Syntax highlighting (syntect),
  LaTeX → MathML (pulldown-latex), and tweet embeds are all fully resolved
  into plain HTML during `build`. None of it ships as client-side
  JavaScript or waits on a third-party script at request time.
- **One inlined stylesheet, zero extra requests.** CSS is compiled into the
  binary and inlined into every page's `<head>` — no separate stylesheet
  fetch, no flash of unstyled content.
- **Images are optimized, not just copied.** Every image is downscaled and
  re-encoded to WebP at build time, with intrinsic `width`/`height` and lazy
  loading set automatically — a 2.4 MB screenshot ships as ~25 KB, and the
  full-resolution original is still one click away.
- **Almost no client-side JavaScript.** The only script on a page by default
  is a ~1.3 KB inline theme toggle; the rest is plain HTML and CSS. (Opting
  into giscus comments is the one exception — that pulls in a third-party
  script on post pages.)
- **Small, self-contained pages.** The post rendered above — code block,
  display math, and a references section — is ~22 KB of HTML, CSS included,
  in a single request.

## Usage

Run these from the root of a site directory (one containing `content/`):

```sh
# Install a specific tagged release (recommended — see Releasing below)
cargo install --git https://github.com/alcazar90/rustoky --tag v0.1.0 --locked

# Or track the default branch instead of a pinned release
cargo install --git https://github.com/alcazar90/rustoky --locked

# Build content/ -> public/
rustoky build

# Build including draft: true posts, for local preview only
rustoky build --drafts

# Build (optionally --drafts), then serve public/ at http://127.0.0.1:8000/
rustoky serve --drafts
rustoky serve --port 3000

# Scaffold a new post at content/posts/YYYY-MM-DD-<slug>.md
rustoky new-post "My Post Title"
```

For local development against this repo instead of a published version:

```sh
cargo install --path /path/to/rustoky --locked
```

## What a site needs

- `content/config.toml` — site config (see `src/config.rs` for the schema).
- `content/posts/*.md`, `content/pages/*.md` — Markdown with YAML or TOML
  frontmatter (`title`, `date`, `slug`, `tags`, `description`, `draft`,
  `lang`).
- `content/static/` — copied verbatim into `public/`; images are optimized to
  WebP derivatives at build time (requires `cwebp`/`gif2webp` on `PATH`, e.g.
  `brew install webp` / `apt-get install -y webp` — the build still succeeds
  without them, just ships images unoptimized).
- Optional `content/posts/<post-stem>.refs.yaml` bibliography sidecars for
  `\cite{key}`/`\citep{key}`.

Templates and CSS are compiled into the binary (`include_str!`) — there's no
runtime template loading or per-site theming. If you want a different look,
fork this repo.

## Example

`example/` is a small demo site (config, two posts, a page, a bibliography
sidecar) used to exercise and preview the generator. Build and serve it the
same way you would any other site, pointing `cargo run` at this repo's
manifest:

```sh
cd example
cargo run --manifest-path ../Cargo.toml --release -- serve --port 8123
```

Then open `http://127.0.0.1:8123/`. The home page lists both demo posts:

![The demo site's home page: title, description, and a list of posts, in Flexoki's dark theme](example/screenshots/home.png)

`example/content/config.toml` shows the minimal config needed to get a site
running — title, url, author, description, and a menu. `avatar` and
`[social]` are both optional; leaving them out (as the example does) drops
the avatar image and lays the social-links row out without it.

## Development

```sh
cargo test
cargo fmt
cargo clippy
```

### Releasing

`Cargo.toml`'s `version` is the only source of truth — no changelog file,
no release tooling. To cut a release:

```sh
# 1. Bump `version` in Cargo.toml, commit it
git commit -am "chore: bump version to 0.2.0"

# 2. Tag it
git tag -a v0.2.0 -m "v0.2.0"
git push origin main --tags

# 3. In a consuming site, pin to the new tag
cargo install --git https://github.com/alcazar90/rustoky --tag v0.2.0 --locked
```

Pre-1.0, don't over-think semver strictness: bump the middle number for any
notable change (feature or breaking), the last for trivial fixes. A
consuming site should always pin `--tag`, never track a branch — that way
shipping a new rustoky feature is a deliberate one-line version bump in that
site's own repo, not something that happens silently on its next unrelated
deploy.

## Credits

The default color palette is [Flexoki](https://github.com/kepano/flexoki) by
[Steph Ango](https://stephango.com/flexoki), used under the MIT License. 

## License

MIT
