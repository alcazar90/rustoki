//! Footnotes rendered as sidenotes.
//!
//! Two author syntaxes converge on one rendering. A manuscript's
//! `\footnote{…}` is rewritten into a CommonMark footnote — `[^fn-N]` at the
//! reference site, `[^fn-N]: …` appended as a definition — *before* the
//! markdown parser runs, so from that point on it is indistinguishable from a
//! hand-written `[^key]` footnote. Both then take the identical path through
//! the pipeline: a note's body is walked by the same `transform_events` as
//! the prose, so math, citations, links and code inside a note render exactly
//! as they do outside one, with no second renderer to keep in sync.
//!
//! The reference site emits an HTML-comment placeholder rather than the final
//! markup, because pulldown-cmark reaches a definition *after* every
//! reference to it. `splice` fills the placeholders in once the whole
//! document has been walked, in the same spirit as `assets::rewrite_images`.
//!
//! Presentation is a numbered mark plus its note: the stylesheet typesets the
//! note in the right gutter on viewports wide enough to have one, and
//! collapses it into a click-to-open block everywhere else (see "Sidenotes"
//! in `styles/main.css`). The disclosure is a checkbox — no script, and no
//! `#fragment` in the URL, so opening a note on a phone neither scrolls the
//! page nor adds a history entry.
//!
//! Nothing here fails the build: an unbalanced `\footnote{` is left as
//! written, and a reference with no definition degrades to a bare number.

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Marker that opens a LaTeX footnote. The trailing `{` is part of the match
/// so `balanced_body` can start scanning at the byte right after it.
const MARKER: &str = "\\footnote{";

// ── LaTeX → CommonMark normalization ─────────────────────────────────────────

/// Rewrite every `\footnote{…}` into a CommonMark footnote reference, and
/// append the corresponding definitions to the end of the body.
///
/// Brace matching is done by scanning, not by regex: footnote bodies in a
/// real manuscript contain `\frac{a}{b}`, `\text{…}` and other nested groups,
/// and a `\{[^}]+\}` pattern would truncate at the first inner `}`.
pub fn normalize_latex_footnotes(body: &str) -> String {
    if !body.contains(MARKER) {
        return body.to_string();
    }

    let protected = protected_ranges(body);
    let mut out = String::with_capacity(body.len());
    let mut defs = String::new();
    let mut n = 0u32;
    let mut cursor = 0usize;

    while let Some(rel) = body[cursor..].find(MARKER) {
        let start = cursor + rel;
        let after_marker = start + MARKER.len();

        // A post *about* LaTeX shows `\footnote{…}` in a code span or fence;
        // that has to survive as literal text.
        if is_protected(start, &protected) {
            out.push_str(&body[cursor..after_marker]);
            cursor = after_marker;
            continue;
        }

        match balanced_body(body, after_marker) {
            Some((inner_end, after)) => {
                n += 1;
                out.push_str(&body[cursor..start]);
                out.push_str(&format!("[^fn-{n}]"));
                defs.push_str(&format!(
                    "\n[^fn-{n}]: {}\n",
                    flatten(&body[after_marker..inner_end])
                ));
                cursor = after;
            }
            // Unbalanced braces: leave the source exactly as written rather
            // than swallowing the rest of the post into a footnote.
            None => {
                out.push_str(&body[cursor..after_marker]);
                cursor = after_marker;
            }
        }
    }
    out.push_str(&body[cursor..]);

    if n > 0 {
        out.push_str("\n\n");
        out.push_str(&defs);
    }
    out
}

/// Byte ranges that `normalize_latex_footnotes` must not rewrite: fenced code
/// blocks (including their fence lines) and inline code spans.
fn protected_ranges(body: &str) -> Vec<(usize, usize)> {
    static CODE_SPAN: OnceLock<Regex> = OnceLock::new();
    let span_re = CODE_SPAN.get_or_init(|| Regex::new(r"``[^`]*``|`[^`\n]*`").unwrap());

    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut fence: Option<&'static str> = None;
    let mut offset = 0usize;

    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let opens = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };

        match fence {
            // Inside a fence: everything is protected until the closing
            // marker of the same kind.
            Some(marker) => {
                ranges.push((offset, offset + line.len()));
                if opens == Some(marker) {
                    fence = None;
                }
            }
            None => match opens {
                Some(marker) => {
                    fence = Some(marker);
                    ranges.push((offset, offset + line.len()));
                }
                None => {
                    for m in span_re.find_iter(line) {
                        ranges.push((offset + m.start(), offset + m.end()));
                    }
                }
            },
        }
        offset += line.len();
    }
    ranges
}

fn is_protected(pos: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(start, end)| pos >= start && pos < end)
}

/// Find the `}` closing the group opened just before `from`, honouring
/// nesting and LaTeX's `\{` / `\}` escapes. Returns `(index_of_closing_brace,
/// index_after_it)`, or `None` if the group is never closed.
fn balanced_body(body: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = body.as_bytes();
    let mut depth = 1usize;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            // `\{` and `\}` are literal braces, not group delimiters. Skipping
            // two bytes can land mid-codepoint, which is harmless: every UTF-8
            // continuation byte is >= 0x80 and so matches none of the arms.
            b'\\' => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Collapse a footnote body onto one line. A CommonMark footnote definition
/// continues across lines only when they are indented; flattening sidesteps
/// that entirely, and a LaTeX footnote is a single paragraph by construction.
fn flatten(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── Collection and markup ────────────────────────────────────────────────────

/// Footnotes gathered during one document walk.
#[derive(Debug, Default)]
pub struct Footnotes {
    /// Rendered, phrasing-safe body HTML per definition key.
    defs: HashMap<String, String>,
    /// Definition keys in order of first *reference*, so a note is numbered
    /// by where it is cited in the prose, not by where it is defined.
    order: Vec<String>,
    numbers: HashMap<String, usize>,
}

impl Footnotes {
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Record a reference to `key` and return the placeholder to emit in its
    /// place. Resolved later by `splice`.
    pub fn placeholder(&mut self, key: &str) -> String {
        format!("<!--rustoki-sn:{}-->", self.number_of(key))
    }

    /// Store a definition's rendered body, converted to phrasing content.
    pub fn define(&mut self, key: &str, body_html: &str) {
        self.defs.insert(key.to_string(), phrasing(body_html));
    }

    fn number_of(&mut self, key: &str) -> usize {
        if let Some(&n) = self.numbers.get(key) {
            return n;
        }
        self.order.push(key.to_string());
        let n = self.order.len();
        self.numbers.insert(key.to_string(), n);
        n
    }

    fn markup(&self, n: usize) -> String {
        let body = n
            .checked_sub(1)
            .and_then(|i| self.order.get(i))
            .and_then(|key| self.defs.get(key));

        match body {
            Some(body) => format!(
                "<input type=\"checkbox\" id=\"sn-{n}\" class=\"sn-toggle\">\
                 <label class=\"sn-mark\" for=\"sn-{n}\" aria-label=\"Footnote {n}\">{n}</label>\
                 <span class=\"sidenote\" role=\"doc-footnote\">\
                 <span class=\"sn-num\" aria-hidden=\"true\">{n}</span>{body}</span>"
            ),
            // Referenced but never defined. Keep the mark so the prose still
            // reads, drop the note: a content-level problem degrades.
            None => format!("<sup class=\"sn-orphan\">{n}</sup>"),
        }
    }
}

/// Replace every reference placeholder with its sidenote markup.
pub fn splice(html: &str, footnotes: &Footnotes) -> String {
    if footnotes.is_empty() {
        return html.to_string();
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"<!--rustoki-sn:(\d+)-->").unwrap());
    re.replace_all(html, |caps: &regex::Captures| {
        match caps[1].parse::<usize>() {
            Ok(n) => footnotes.markup(n),
            Err(_) => String::new(),
        }
    })
    .into_owned()
}

/// Turn a rendered note body into valid *phrasing* content.
///
/// The reference sits mid-paragraph, so the note element has to be a `<span>`
/// — an HTML parser confronted with a `<p>` or `<div>` inside one closes the
/// enclosing paragraph and hoists the block out, taking the float with it and
/// wrecking the layout. pulldown-cmark wraps a definition body in `<p>`, and
/// display math renders as a `<div>`, so both become spans here; `.sn-p` and
/// the existing `.math.display` rule restore block layout in CSS. A `<pre>`
/// code block inside a note is not supported.
fn phrasing(html: &str) -> String {
    let trimmed = html.trim();
    let single_paragraph = trimmed.starts_with("<p>")
        && trimmed.ends_with("</p>")
        && !trimmed[3..trimmed.len() - 4].contains("<p>");

    let body = if single_paragraph {
        trimmed[3..trimmed.len() - 4].trim()
    } else {
        trimmed
    };

    body.replace("<p>", "<span class=\"sn-p\">")
        .replace("</p>", "</span>")
        .replace("<div>", "<span>")
        .replace("<div ", "<span ")
        .replace("</div>", "</span>")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_footnote_becomes_reference_and_definition() {
        let out = normalize_latex_footnotes(r"Reward\footnote{Only for episodic tasks.} is noisy.");
        assert!(out.starts_with("Reward[^fn-1] is noisy."), "got: {out}");
        assert!(
            out.contains("[^fn-1]: Only for episodic tasks."),
            "got: {out}"
        );
    }

    #[test]
    fn nested_braces_are_matched_not_truncated() {
        let out = normalize_latex_footnotes(r"x\footnote{The same holds for $\frac{a}{b}$ too.}");
        assert!(
            out.contains(r"[^fn-1]: The same holds for $\frac{a}{b}$ too."),
            "got: {out}"
        );
        // Nothing of the original command may survive.
        assert!(!out.contains("\\footnote"), "got: {out}");
    }

    #[test]
    fn escaped_braces_do_not_close_the_group() {
        let out = normalize_latex_footnotes(r"x\footnote{A literal \} brace.} y");
        assert!(out.contains("x[^fn-1] y"), "got: {out}");
        assert!(out.contains(r"[^fn-1]: A literal \} brace."), "got: {out}");
    }

    #[test]
    fn multiple_footnotes_are_numbered_in_order() {
        let out = normalize_latex_footnotes(r"a\footnote{first} b\footnote{second}");
        assert!(out.contains("a[^fn-1] b[^fn-2]"), "got: {out}");
        assert!(out.contains("[^fn-1]: first"), "got: {out}");
        assert!(out.contains("[^fn-2]: second"), "got: {out}");
    }

    #[test]
    fn multiline_footnote_body_is_flattened_to_one_line() {
        let out = normalize_latex_footnotes("x\\footnote{first line\n  second line} y");
        assert!(
            out.contains("[^fn-1]: first line second line"),
            "got: {out}"
        );
    }

    #[test]
    fn footnote_inside_code_span_is_left_alone() {
        let src = "Write `\\footnote{note}` to add one.";
        assert_eq!(normalize_latex_footnotes(src), src);
    }

    #[test]
    fn footnote_inside_fenced_block_is_left_alone() {
        let src = "text\n\n```latex\n\\footnote{not a note}\n```\n\nmore";
        assert_eq!(normalize_latex_footnotes(src), src);
    }

    #[test]
    fn footnote_after_a_fence_is_still_rewritten() {
        let src = "```\n\\footnote{ignored}\n```\n\nreal\\footnote{note} here";
        let out = normalize_latex_footnotes(src);
        assert!(out.contains("\\footnote{ignored}"), "got: {out}");
        assert!(out.contains("real[^fn-1] here"), "got: {out}");
        assert!(out.contains("[^fn-1]: note"), "got: {out}");
    }

    #[test]
    fn unbalanced_footnote_is_left_as_written() {
        let src = r"broken\footnote{never closed";
        assert_eq!(normalize_latex_footnotes(src), src);
    }

    #[test]
    fn body_without_footnotes_is_untouched() {
        let src = "Just prose with $math$ and \\cite{key}.";
        assert_eq!(normalize_latex_footnotes(src), src);
    }

    #[test]
    fn numbers_follow_first_reference_not_definition_order() {
        let mut fns = Footnotes::default();
        // `b` is referenced first, so it must be note 1.
        let first = fns.placeholder("b");
        let second = fns.placeholder("a");
        let repeat = fns.placeholder("b");
        assert_eq!(first, "<!--rustoki-sn:1-->");
        assert_eq!(second, "<!--rustoki-sn:2-->");
        assert_eq!(repeat, "<!--rustoki-sn:1-->");
    }

    #[test]
    fn splice_builds_toggle_markup() {
        let mut fns = Footnotes::default();
        let placeholder = fns.placeholder("a");
        fns.define("a", "<p>The note body.</p>");
        let html = splice(&format!("<p>Text{placeholder}.</p>"), &fns);
        assert!(html.contains(r#"<input type="checkbox" id="sn-1" class="sn-toggle">"#), "got: {html}");
        assert!(html.contains(r#"<label class="sn-mark" for="sn-1""#), "got: {html}");
        assert!(html.contains(r#"<span class="sidenote" role="doc-footnote">"#), "got: {html}");
        assert!(html.contains("The note body."), "got: {html}");
        assert!(!html.contains("rustoki-sn"), "placeholder leaked: {html}");
    }

    #[test]
    fn reference_without_definition_degrades_to_a_bare_number() {
        let mut fns = Footnotes::default();
        let placeholder = fns.placeholder("ghost");
        let html = splice(&placeholder, &fns);
        assert_eq!(html, r#"<sup class="sn-orphan">1</sup>"#);
    }

    #[test]
    fn splice_without_footnotes_returns_input() {
        let fns = Footnotes::default();
        assert_eq!(splice("<p>plain</p>", &fns), "<p>plain</p>");
    }

    #[test]
    fn phrasing_unwraps_a_single_paragraph() {
        assert_eq!(phrasing("<p>one paragraph</p>\n"), "one paragraph");
    }

    #[test]
    fn phrasing_converts_blocks_to_spans() {
        let out = phrasing("<p>a</p>\n<p>b</p>\n");
        assert!(!out.contains("<p>"), "got: {out}");
        assert_eq!(out.matches(r#"<span class="sn-p">"#).count(), 2, "got: {out}");
    }

    #[test]
    fn phrasing_converts_display_math_div_to_span() {
        let out = phrasing(r#"<p>see <div class="math display">M</div></p>"#);
        assert!(!out.contains("<div"), "got: {out}");
        assert!(out.contains(r#"<span class="math display">"#), "got: {out}");
    }
}
