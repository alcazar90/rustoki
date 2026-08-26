//! Site configuration loaded from `content/config.toml`.
//!
//! The schema is intentionally flat — no nested sections — so a missing
//! field is a typo, not a structural mismatch. Add fields here as the
//! generator grows.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuItem {
    pub name: String,
    pub url: String,
}

/// Optional giscus comments configuration. When present, post pages render a
/// giscus mount; when absent, post pages render without a comments section.
/// All fields are required when the block is present — `repo_id` and
/// `category_id` are issued by https://giscus.app for a given GitHub repo.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GiscusConfig {
    pub repo: String,
    pub repo_id: String,
    pub category: String,
    pub category_id: String,
    pub mapping: String,
    pub reactions_enabled: String,
    pub input_position: String,
    pub strict: String,
    pub loading: String,
}

/// Optional margin-figure configuration. Present enables the pixel traveller
/// who crosses the side gutter; absent ships none of it — no atlas, no
/// stylesheet, no script. Every field has a default, so `[margin]` on its own
/// is a complete configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarginConfig {
    /// Which cast member to draw. See `margin::cast::CAST` for the names an
    /// unknown value is reported against.
    #[serde(default = "default_character")]
    pub character: String,
    /// CSS pixels per sprite pixel. Fractional is fine and stays crisp on the
    /// 2x displays this is mostly seen on.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Narrowest viewport that still has gutter for the figure. Defaults to
    /// whatever the chosen character at the chosen scale actually needs, which
    /// is nearly always the right answer.
    #[serde(default)]
    pub min_width: Option<u32>,
    /// Seconds before the first crossing. Long enough that it never reads as
    /// part of the page arriving.
    #[serde(default = "default_first_delay")]
    pub first_delay: u32,
    /// Seconds between crossings, picked uniformly from this range so it never
    /// syncs to scrolling or to itself.
    #[serde(default = "default_interval")]
    pub interval: [u32; 2],
}

fn default_character() -> String {
    "traveller".to_string()
}
fn default_scale() -> f32 {
    2.5
}
fn default_first_delay() -> u32 {
    90
}
fn default_interval() -> [u32; 2] {
    [240, 420]
}

impl Default for MarginConfig {
    fn default() -> Self {
        Self {
            character: default_character(),
            scale: default_scale(),
            min_width: None,
            first_delay: default_first_delay(),
            interval: default_interval(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SocialLinks {
    #[serde(default)]
    pub cv: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub x: String,
    #[serde(default)]
    pub goodreads: String,
    #[serde(default)]
    pub linkedin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub title: String,
    pub url: String,
    pub author: String,
    pub description: String,
    #[serde(default)]
    pub footnote: String,
    /// Optional path/URL to a profile image shown on the home page. Absent
    /// (the default) omits the avatar image; the social links still render,
    /// just without the avatar's reserved space.
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub social: SocialLinks,
    #[serde(default)]
    pub menu: Vec<MenuItem>,
    /// Optional. Absent in dev/local configs that haven't claimed a giscus
    /// repo yet — post pages just skip the comments mount in that case.
    #[serde(default)]
    pub giscus: Option<GiscusConfig>,
    /// Optional. Absent means the site ships no margin figure at all.
    #[serde(default)]
    pub margin: Option<MarginConfig>,
}

impl Config {
    /// Read and parse a TOML config file. The caller is responsible for
    /// pointing this at `content/config.toml` (or wherever the site root is).
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        let config: Self = toml::from_str(&raw)
            .with_context(|| format!("parsing TOML in {}", path.display()))?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(contents: &str) -> tempfile_lite::TempPath {
        let mut t = tempfile_lite::NamedTempFile::new("ssg-config", ".toml");
        t.file.write_all(contents.as_bytes()).unwrap();
        t.into_path()
    }

    #[test]
    fn loads_flat_schema() {
        let path = write_temp(
            r#"
title = "My Site"
url = "https://example.com"
author = "Jane"
description = "A blog"

[[menu]]
name = "About"
url = "/about"

[[menu]]
name = "Posts"
url = "/posts"
"#,
        );

        let config = Config::load(&path).unwrap();
        assert_eq!(config.title, "My Site");
        assert_eq!(config.url, "https://example.com");
        assert_eq!(config.author, "Jane");
        assert_eq!(config.description, "A blog");
        assert_eq!(config.menu.len(), 2);
        assert_eq!(config.menu[0].name, "About");
        assert_eq!(config.menu[1].url, "/posts");
    }

    #[test]
    fn empty_menu_is_ok() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"
"#,
        );
        let config = Config::load(&path).unwrap();
        assert!(config.menu.is_empty());
    }

    #[test]
    fn giscus_block_is_optional() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"
"#,
        );
        let config = Config::load(&path).unwrap();
        assert!(config.giscus.is_none());
    }

    #[test]
    fn giscus_block_parses_when_present() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"

[giscus]
repo = "owner/repo"
repo_id = "R_kgABC"
category = "Comments"
category_id = "DIC_kwABC"
mapping = "pathname"
reactions_enabled = "1"
input_position = "bottom"
strict = "0"
loading = "lazy"
"#,
        );
        let config = Config::load(&path).unwrap();
        let g = config.giscus.expect("giscus block should parse");
        assert_eq!(g.repo, "owner/repo");
        assert_eq!(g.repo_id, "R_kgABC");
        assert_eq!(g.category, "Comments");
        assert_eq!(g.category_id, "DIC_kwABC");
        assert_eq!(g.mapping, "pathname");
        assert_eq!(g.reactions_enabled, "1");
        assert_eq!(g.input_position, "bottom");
        assert_eq!(g.strict, "0");
        assert_eq!(g.loading, "lazy");
    }

    #[test]
    fn margin_block_is_optional() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"
"#,
        );
        assert!(Config::load(&path).unwrap().margin.is_none());
    }

    #[test]
    fn bare_margin_block_is_a_complete_configuration() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"

[margin]
"#,
        );
        let m = Config::load(&path).unwrap().margin.expect("margin should parse");
        assert_eq!(m.character, "traveller");
        assert_eq!(m.first_delay, 90);
        assert_eq!(m.interval, [240, 420]);
        assert!(m.min_width.is_none());
    }

    #[test]
    fn margin_fields_override_individually() {
        let path = write_temp(
            r#"
title = "T"
url = "U"
author = "A"
description = "D"

[margin]
character = "someone-else"
scale = 3.0
min_width = 1200
interval = [60, 90]
"#,
        );
        let m = Config::load(&path).unwrap().margin.unwrap();
        assert_eq!(m.character, "someone-else");
        assert_eq!(m.scale, 3.0);
        assert_eq!(m.min_width, Some(1200));
        assert_eq!(m.interval, [60, 90]);
        // untouched fields keep their defaults
        assert_eq!(m.first_delay, 90);
    }

    #[test]
    fn missing_file_reports_clear_error() {
        let err = Config::load("/nonexistent/path/to/config.toml").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("reading config"), "got: {msg}");
    }
}

#[cfg(test)]
mod tempfile_lite {
    //! Tiny inline temp-file helper to avoid pulling in a tempfile crate.
    use std::fs::{self, File};
    use std::ops::Deref;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct NamedTempFile {
        pub path: PathBuf,
        pub file: File,
    }

    pub struct TempPath(PathBuf);

    impl Deref for TempPath {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for TempPath {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    impl NamedTempFile {
        pub fn new(prefix: &str, suffix: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let pid = std::process::id();
            let path = std::env::temp_dir().join(format!("{prefix}-{pid}-{n}{suffix}"));
            let file = File::create(&path).expect("create tempfile");
            Self { path, file }
        }

        pub fn into_path(self) -> TempPath {
            TempPath(self.path)
        }
    }
}
