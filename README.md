# rustoky

A small, opinionated static site generator written in Rust. Markdown in,
static HTML out — with syntax highlighting, LaTeX math, figures, a
bibliography system, and a [Flexoki](https://github.com/kepano/flexoki)-[themed](https://stephango.com/about#colophon) default look baked in.

Built to power a single personal blog; extracted here so the generator can be
installed and reused independently of that blog's content.

![A rendered post page: syntax-highlighted code, LaTeX math, and a numbered references section, in Flexoki's dark theme](example/screenshots/post.png)

## Usage

Run these from the root of a site directory (one containing `content/`):

```sh
# Install the binary from this repo
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

## Credits

The default color palette is [Flexoki](https://github.com/kepano/flexoki) by
[Steph Ango](https://stephango.com/flexoki), used under the MIT License. 

## License

MIT
