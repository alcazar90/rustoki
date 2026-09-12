//! Algorithm blocks: a ```` ```algorithm ```` fence rendered as a numbered,
//! captioned pseudocode listing in the manner of LaTeX's `algorithm` /
//! `algorithmic` packages.
//!
//! ````text
//! ```algorithm
//! title: Vanilla policy gradient
//! label: alg:reinforce
//! ---
//! Input: policy $\pi_\theta$, learning rate $\alpha$
//! for iteration $= 0, 1, 2, \dots, N$ do
//!     Collect trajectories $\mathcal{D}$ by running $\pi_\theta$
//!     $\theta \leftarrow \theta + \alpha \hat{g}$   // gradient step
//! end for
//! ```
//! ````
//!
//! The header (everything above a `---` line, optional) names the block; the
//! rest is one statement per line, indented to nest. A line is ordinary
//! inline markdown, and it renders by exactly the rules the prose does,
//! because the caller hands `render` the prose's own inline renderer rather
//! than this module growing a second one: math, code spans, emphasis, links
//! and citations inside a statement all come out as they would in a
//! paragraph. What this module adds is the structure around that text: line
//! numbers, indentation depth, bold keywords (`for … do`, `if … then`,
//! `end if`, `return`, …), small-caps names after `function` / `procedure`,
//! `// comments`, and a caption numbered in document order. `\algref{label}`
//! anywhere in the post links to the block, with the same `[?:key]` marker
//! for an unknown key that `\figref` uses.
//!
//! The one thing a statement cannot do that a paragraph can is carry raw
//! inline HTML: `<` and `>` are escaped before the markdown parser sees them,
//! so `if a<b and c>d then` reads as written instead of opening a `<b>` tag.
//! The anchors the build's own preprocessors leave in the source (citations,
//! `\ref`, `\figref`) are recognised and let through.
//!
//! Nothing here fails the build: a header is only a header when every line
//! above the `---` is a known key, and a line no rule matches is simply a
//! plain statement.

use super::html_escape;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Is this fence info string an algorithm block?
pub fn is_fence(info: &str) -> bool {
    info.split_whitespace().next() == Some("algorithm")
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// One parsed algorithm fence.
#[derive(Debug, Default, PartialEq)]
pub struct Block {
    pub title: Option<String>,
    pub label: Option<String>,
    pub lines: Vec<Line>,
}

/// One statement. `text` has its indentation removed and is empty for a
/// blank spacer line.
#[derive(Debug, PartialEq)]
pub struct Line {
    pub depth: usize,
    pub text: String,
}

/// Split a fence body into header fields and indented statements.
///
/// Depth is measured in units of the smallest indent the block uses (a tab
/// counts as four columns), so two-space and four-space authors both get one
/// level per step.
pub fn parse(source: &str) -> Block {
    let mut block = Block::default();
    let all: Vec<&str> = source.lines().collect();

    let body_start = match header_end(&all) {
        Some(end) => {
            for field in &all[..end] {
                let field = field.trim();
                if field.is_empty() {
                    continue;
                }
                let (key, value) = field.split_once(':').unwrap_or((field, ""));
                let value = unquote(value.trim());
                let slot = match key.trim().to_ascii_lowercase().as_str() {
                    "title" => &mut block.title,
                    "label" => &mut block.label,
                    // `header_end` admits only these two keys.
                    _ => continue,
                };
                *slot = Some(value.to_string()).filter(|s| !s.is_empty());
            }
            end + 1
        }
        None => 0,
    };

    // Drop blank lines at either end; the ones in between stay as spacers.
    let mut body = &all[body_start..];
    while body.first().is_some_and(|l| l.trim().is_empty()) {
        body = &body[1..];
    }
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body = &body[..body.len() - 1];
    }

    let indents: Vec<usize> = body.iter().map(|l| indent_columns(l)).collect();
    let unit = body
        .iter()
        .zip(&indents)
        .filter(|(line, cols)| **cols > 0 && !line.trim().is_empty())
        .map(|(_, cols)| *cols)
        .min()
        .unwrap_or(1);

    for (line, cols) in body.iter().zip(&indents) {
        let text = line.trim();
        block.lines.push(Line {
            depth: if text.is_empty() { 0 } else { cols / unit },
            text: text.to_string(),
        });
    }
    block
}

/// Index of the `---` line closing a header, if the block has one. Every
/// line above it must be one of the known keys for it to count: a body that
/// opens with `Input:` / `Output:` and then draws a `---` rule keeps those
/// as statements, and a misspelt key shows up on the page as a statement
/// rather than vanishing into a warning.
fn header_end(lines: &[&str]) -> Option<usize> {
    static KEY_RE: OnceLock<Regex> = OnceLock::new();
    let key_re = KEY_RE.get_or_init(|| Regex::new(r"(?i)^\s*(title|label)\s*:").unwrap());
    let end = lines.iter().position(|l| l.trim() == "---")?;
    lines[..end]
        .iter()
        .all(|l| l.trim().is_empty() || key_re.is_match(l))
        .then_some(end)
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn indent_columns(line: &str) -> usize {
    line.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

// ── Numbering ────────────────────────────────────────────────────────────────

/// Algorithm numbers handed out during one document walk, and the labels
/// that claimed them, for `replace_algrefs` to resolve afterwards.
#[derive(Debug, Default)]
pub struct Algorithms {
    count: u32,
    labels: HashMap<String, u32>,
}

impl Algorithms {
    /// The next number in document order. The first block to use a label
    /// keeps it, matching how duplicate figure ids are treated.
    pub fn assign(&mut self, label: Option<&str>) -> u32 {
        self.count += 1;
        if let Some(key) = label {
            self.labels.entry(key.to_string()).or_insert(self.count);
        }
        self.count
    }

    pub fn labels(&self) -> &HashMap<String, u32> {
        &self.labels
    }
}

/// Replace `\algref{key}` in rendered HTML with a link to the block, e.g.
/// `<a href="#key">Algorithm 2</a>`; an unknown key renders as `[?:key]`.
///
/// Runs over the finished HTML rather than the source so the numbers come
/// from the walk that emitted the blocks, not from a second scan that could
/// disagree with it. Text inside `<code>` is left alone, so a post can show
/// the syntax.
pub fn replace_algrefs(html: &str, labels: &HashMap<String, u32>) -> String {
    static ALGREF_RE: OnceLock<Regex> = OnceLock::new();
    if !html.contains("\\algref{") {
        return html.to_string();
    }
    let re = ALGREF_RE.get_or_init(|| Regex::new(r"\\algref\{([^}]+)\}").unwrap());
    let resolve = |caps: &regex::Captures| {
        let key = &caps[1];
        match labels.get(key) {
            Some(n) => format!(r##"<a href="#{key}">Algorithm {n}</a>"##),
            None => format!("[?:{key}]"),
        }
    };

    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find("<code") {
        out.push_str(&re.replace_all(&rest[..open], resolve));
        let code = &rest[open..];
        let end = code
            .find("</code>")
            .map_or(code.len(), |i| i + "</code>".len());
        out.push_str(&code[..end]);
        rest = &code[end..];
    }
    out.push_str(&re.replace_all(rest, resolve));
    out
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// Render a parsed block as a `<figure class="algorithm">`.
///
/// `inline` renders one line of markdown to HTML with no paragraph wrapper;
/// the caller supplies the prose renderer so a statement and a paragraph can
/// never drift apart.
pub fn render(block: &Block, number: u32, inline: &mut dyn FnMut(&str) -> String) -> String {
    let mut html = String::new();
    let id_attr = block
        .label
        .as_deref()
        .map(|key| format!(" id=\"{}\"", html_escape(key)))
        .unwrap_or_default();
    html.push_str(&format!(
        "<figure class=\"algorithm\"{id_attr}>\n<figcaption><span class=\"alg-number\">Algorithm {number}</span>"
    ));
    if let Some(title) = &block.title {
        html.push(' ');
        html.push_str(&inline(&format!(
            "<span class=\"alg-title\">{}</span>",
            markdown_safe(title, false)
        )));
    }
    html.push_str("</figcaption>\n<ol class=\"alg-body\">\n");

    for line in &block.lines {
        if line.text.is_empty() {
            html.push_str("<li class=\"alg-gap\" aria-hidden=\"true\"></li>\n");
            continue;
        }
        let statement = mark_up(&line.text);
        let class = if statement.unnumbered {
            " class=\"alg-head\""
        } else {
            ""
        };
        let depth = if line.depth > 0 {
            format!(" style=\"--depth:{}\"", line.depth)
        } else {
            String::new()
        };
        // The wrapper span (and `alg-title` above) is what keeps the text a
        // paragraph: a line that opens with inline HTML followed by text can
        // start no other CommonMark block, so `# comment`, `- item` or
        // `1. step` at the head of a statement stay literal instead of
        // becoming a heading or a list.
        html.push_str(&format!(
            "<li{class}{depth}>{}</li>\n",
            inline(&format!(
                "<span class=\"alg-stmt\">{}</span>",
                statement.markdown
            ))
        ));
    }

    html.push_str("</ol>\n</figure>\n");
    html
}

struct Statement {
    /// Markdown for the inline renderer, keyword spans already in place.
    markdown: String,
    /// `Input:` / `Output:` style lines carry no line number, as
    /// `\Require` / `\Ensure` do not.
    unnumbered: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// `Input:`, `Ensure:` … — matched only with the colon, which is what
    /// keeps a sentence starting with "Initialize" a plain statement.
    Head,
    Control,
    /// A loop head, inside which `in` / `to` / `downto` are keywords too.
    For,
    /// `function` / `procedure`: the name that follows is set in small caps.
    Function,
}

/// Leading keywords. Longer spellings come before their prefixes so
/// `for each` and `end for` win over `for` and `end`.
const LEADING: &[(&str, Kind)] = &[
    ("for each", Kind::For),
    ("for all", Kind::For),
    ("foreach", Kind::For),
    ("forall", Kind::For),
    ("for", Kind::For),
    ("else if", Kind::Control),
    ("elseif", Kind::Control),
    ("elif", Kind::Control),
    ("else", Kind::Control),
    ("end for", Kind::Control),
    ("end while", Kind::Control),
    ("end if", Kind::Control),
    ("end function", Kind::Control),
    ("end procedure", Kind::Control),
    ("end loop", Kind::Control),
    ("end", Kind::Control),
    ("while", Kind::Control),
    ("if", Kind::Control),
    ("repeat", Kind::Control),
    ("until", Kind::Control),
    ("loop", Kind::Control),
    ("return", Kind::Control),
    ("break", Kind::Control),
    ("continue", Kind::Control),
    ("function", Kind::Function),
    ("procedure", Kind::Function),
    ("input", Kind::Head),
    ("output", Kind::Head),
    ("require", Kind::Head),
    ("ensure", Kind::Head),
    ("data", Kind::Head),
    ("result", Kind::Head),
    ("given", Kind::Head),
    ("initialize", Kind::Head),
    ("initialization", Kind::Head),
    ("parameters", Kind::Head),
    ("hyperparameters", Kind::Head),
];

/// Keywords that close a loop or branch head.
const TRAILING: &[&str] = &["do", "then"];

fn keyword(word: &str) -> String {
    format!("<span class=\"alg-kw\">{}</span>", html_escape(word))
}

fn mark_up(text: &str) -> Statement {
    let (statement, comment) = split_comment(text);
    let statement = statement.trim_end();

    let mut markdown = String::new();
    let mut kind = None;
    let mut rest = statement;

    if let Some((word, after, k)) = leading_keyword(statement) {
        markdown.push_str(&keyword(word));
        kind = Some(k);
        rest = after;
        if k == Kind::Function {
            if let Some((name, after_name)) = identifier(rest) {
                markdown.push(' ');
                markdown.push_str(&format!(
                    "<span class=\"alg-name\">{}</span>",
                    html_escape(name)
                ));
                rest = after_name;
            }
        }
    }

    // `do` / `then` close a loop or branch head and nothing else, so
    // "Input: what to do" keeps its last word plain.
    let (middle, trailing) = match kind {
        Some(Kind::For | Kind::Control) => trailing_keyword(rest),
        _ => (rest.trim_end(), None),
    };
    markdown.push_str(&markdown_safe(middle, kind == Some(Kind::For)));
    if let Some(word) = trailing {
        markdown.push(' ');
        markdown.push_str(&keyword(word));
    }
    if let Some(comment) = comment {
        markdown.push_str(&format!(
            " <span class=\"alg-comment\">\u{25B7} {}</span>",
            markdown_safe(comment.trim(), false)
        ));
    }

    Statement {
        markdown,
        unnumbered: kind == Some(Kind::Head),
    }
}

/// Match a keyword at the head of the line. Returns the keyword as the
/// author spelt it (case is preserved), the remainder, and its kind.
fn leading_keyword(text: &str) -> Option<(&str, &str, Kind)> {
    for (kw, kind) in LEADING {
        let n = kw.len();
        if text.len() < n || !text.is_char_boundary(n) || !text[..n].eq_ignore_ascii_case(kw) {
            continue;
        }
        let next = text[n..].chars().next();
        let matched = match kind {
            Kind::Head => next == Some(':'),
            _ => next.map_or(true, |c| c.is_whitespace() || c == ':' || c == '('),
        };
        if matched {
            // A head keyword owns its colon: "Input:" is one token.
            let n = if *kind == Kind::Head { n + 1 } else { n };
            return Some((&text[..n], &text[n..], *kind));
        }
    }
    None
}

/// Split a trailing `do` / `then` off the line.
fn trailing_keyword(text: &str) -> (&str, Option<&str>) {
    let trimmed = text.trim_end();
    for kw in TRAILING {
        let n = kw.len();
        if trimmed.len() > n
            && trimmed.is_char_boundary(trimmed.len() - n)
            && trimmed[trimmed.len() - n..].eq_ignore_ascii_case(kw)
            && trimmed[..trimmed.len() - n].ends_with(char::is_whitespace)
        {
            let cut = trimmed.len() - n;
            return (trimmed[..cut].trim_end(), Some(&trimmed[cut..]));
        }
    }
    (trimmed, None)
}

/// The name following `function` / `procedure`: leading whitespace, then a
/// run of word characters. Anything else (a math name, say) is left to the
/// ordinary inline path.
fn identifier(text: &str) -> Option<(&str, &str)> {
    let start = text.len() - text.trim_start().len();
    let name_len = text[start..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .map(char::len_utf8)
        .sum::<usize>();
    (start > 0 && name_len > 0).then(|| (&text[start..start + name_len], &text[start + name_len..]))
}

/// Split `// comment` off a statement. Only a `//` at the start of the line
/// or after whitespace counts, and only outside math and code spans, so a
/// URL or an integer division inside `$…$` is not mistaken for one.
fn split_comment(text: &str) -> (&str, Option<&str>) {
    for (start, end, raw) in segments(text) {
        if raw {
            continue;
        }
        let mut from = start;
        while let Some(rel) = text[from..end].find("//") {
            let at = from + rel;
            let after_space = at == 0 || text[..at].ends_with(char::is_whitespace);
            if after_space {
                return (&text[..at], Some(&text[at + 2..]));
            }
            from = at + 2;
        }
    }
    (text, None)
}

/// Prepare plain text for the markdown parser: `<` and `>` become entities
/// so no run of characters can open an inline HTML tag, and inside a loop
/// head `in` / `to` / `downto` become keywords. Math, code spans and the
/// build's own anchors pass through untouched.
fn markdown_safe(text: &str, in_loop_head: bool) -> String {
    static LOOP_WORDS: OnceLock<Regex> = OnceLock::new();
    let loop_words = LOOP_WORDS.get_or_init(|| Regex::new(r"\b(in|to|downto)\b").unwrap());

    let mut out = String::with_capacity(text.len());
    for (start, end, raw) in segments(text) {
        let piece = &text[start..end];
        if raw {
            out.push_str(piece);
            continue;
        }
        let escaped = piece.replace('<', "&lt;").replace('>', "&gt;");
        if in_loop_head {
            out.push_str(
                &loop_words.replace_all(&escaped, |caps: &regex::Captures| keyword(&caps[1])),
            );
        } else {
            out.push_str(&escaped);
        }
    }
    out
}

/// Byte ranges of a line, flagged `true` where the text must reach the
/// markdown parser verbatim: math, code spans, and the anchors left behind
/// by citation / `\ref` / `\figref` preprocessing.
fn segments(text: &str) -> Vec<(usize, usize, bool)> {
    static RAW_RE: OnceLock<Regex> = OnceLock::new();
    let raw_re = RAW_RE.get_or_init(|| {
        Regex::new(concat!(
            r"\$\$[^$]+?\$\$",
            r"|\$[^$\n]+?\$",
            r"|``[^`]*``|`[^`\n]*`",
            r##"|<sup><a href="#[^"]*" class="cite">\[\d+\]</a></sup>"##,
            r##"|<a href="#[^"]*">[^<]*</a>"##,
        ))
        .unwrap()
    });
    let mut out = Vec::new();
    let mut last = 0;
    for m in raw_re.find_iter(text) {
        if m.start() > last {
            out.push((last, m.start(), false));
        }
        out.push((m.start(), m.end(), true));
        last = m.end();
    }
    if last < text.len() {
        out.push((last, text.len(), false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stand-in for the prose renderer: markdown in, markdown out. Enough to
    /// see the structure this module adds without parsing anything.
    fn identity(md: &str) -> String {
        md.to_string()
    }

    fn line(depth: usize, text: &str) -> Line {
        Line {
            depth,
            text: text.to_string(),
        }
    }

    #[test]
    fn fence_is_recognised_by_its_first_token() {
        assert!(is_fence("algorithm"));
        assert!(is_fence("algorithm extra"));
        assert!(!is_fence("rust"));
        assert!(!is_fence("algorithms"));
    }

    #[test]
    fn parse_reads_header_and_indents() {
        let block = parse(
            "title: \"Sum\"\nlabel: alg:sum\n---\nInput: $n$\nfor $i = 1$ to $n$ do\n    $s \\leftarrow s + i$\nend for\n",
        );
        assert_eq!(block.title.as_deref(), Some("Sum"));
        assert_eq!(block.label.as_deref(), Some("alg:sum"));
        assert_eq!(
            block.lines,
            vec![
                line(0, "Input: $n$"),
                line(0, "for $i = 1$ to $n$ do"),
                line(1, "$s \\leftarrow s + i$"),
                line(0, "end for"),
            ]
        );
    }

    #[test]
    fn parse_without_header_treats_everything_as_body() {
        let block = parse("Input: $x$\nreturn $x$\n");
        assert_eq!(block.title, None);
        assert_eq!(block.label, None);
        assert_eq!(block.lines.len(), 2);
    }

    #[test]
    fn parse_keeps_a_rule_line_that_is_not_a_header_close() {
        // "Input: $x$" looks like a key, but "a plain statement" does not,
        // so the `---` is a statement, not a header terminator.
        let block = parse("Input: $x$\na plain statement\n---\nreturn\n");
        assert_eq!(block.title, None);
        assert_eq!(block.lines.len(), 4);
        assert_eq!(block.lines[2].text, "---");
    }

    #[test]
    fn parse_does_not_mistake_a_preamble_for_a_header() {
        // `Input:` / `Output:` followed by a `---` rule is a body, not a
        // header, and so is a misspelt key: both stay visible as statements.
        let block = parse("Input: $x$\nOutput: $y$\n---\nreturn $y$\n");
        assert_eq!(block.title, None);
        assert_eq!(block.lines.len(), 4);
        assert_eq!(block.lines[0].text, "Input: $x$");
        let typo = parse("titel: Sum\n---\nreturn\n");
        assert_eq!(typo.title, None);
        assert_eq!(typo.lines[0].text, "titel: Sum");
    }

    #[test]
    fn parse_measures_depth_in_units_of_the_smallest_indent() {
        let two = parse("a\n  b\n    c\n");
        assert_eq!(
            two.lines.iter().map(|l| l.depth).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        let tabs = parse("a\n\tb\n\t\tc\n");
        assert_eq!(
            tabs.lines.iter().map(|l| l.depth).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn parse_keeps_inner_blank_lines_and_drops_outer_ones() {
        let block = parse("\nInput: $x$\n\nreturn $x$\n\n");
        assert_eq!(block.lines.len(), 3);
        assert_eq!(block.lines[1].text, "");
    }

    #[test]
    fn keywords_are_marked_at_head_and_tail() {
        let s = mark_up("for each $x \\in D$ do");
        assert_eq!(
            s.markdown,
            "<span class=\"alg-kw\">for each</span> $x \\in D$ <span class=\"alg-kw\">do</span>"
        );
        assert!(!s.unnumbered);
    }

    #[test]
    fn keyword_case_is_preserved() {
        let s = mark_up("For $i$ Do");
        assert!(s.markdown.starts_with("<span class=\"alg-kw\">For</span>"));
        assert!(s.markdown.ends_with("<span class=\"alg-kw\">Do</span>"));
    }

    #[test]
    fn trailing_keywords_only_close_a_loop_or_branch_head() {
        let plain = mark_up("Input: what to do");
        assert!(!plain.markdown.contains(">do<"), "got: {}", plain.markdown);
        let call = mark_up("Run the update step then");
        assert!(!call.markdown.contains("alg-kw"), "got: {}", call.markdown);
        let branch = mark_up("if $x$ then");
        assert!(branch
            .markdown
            .ends_with("<span class=\"alg-kw\">then</span>"));
    }

    #[test]
    fn head_keywords_need_their_colon_and_go_unnumbered() {
        let with = mark_up("Input: policy $\\pi_\\theta$");
        assert!(with.unnumbered);
        assert!(with
            .markdown
            .starts_with("<span class=\"alg-kw\">Input:</span> policy"));

        let without = mark_up("Initialize policy $\\pi_\\theta$");
        assert!(!without.unnumbered);
        assert!(
            !without.markdown.contains("alg-kw"),
            "got: {}",
            without.markdown
        );
    }

    #[test]
    fn longer_keywords_win_over_their_prefixes() {
        assert!(mark_up("end for").markdown.contains(">end for<"));
        assert!(mark_up("else if $x$ then").markdown.contains(">else if<"));
        assert!(!mark_up("endless loop").markdown.contains("alg-kw"));
        assert!(!mark_up("iff $x$").markdown.contains("alg-kw"));
    }

    #[test]
    fn loop_head_marks_in_and_to_as_keywords() {
        let s = mark_up("for $i = 1$ to $n$ do");
        assert!(
            s.markdown.contains("<span class=\"alg-kw\">to</span>"),
            "got: {}",
            s.markdown
        );
        let plain = mark_up("assign $x$ to $y$");
        assert!(!plain.markdown.contains("alg-kw"));
    }

    #[test]
    fn function_name_is_set_in_small_caps() {
        let s = mark_up("function Reinforce($\\pi_\\theta$, $\\alpha$)");
        assert_eq!(
            s.markdown,
            "<span class=\"alg-kw\">function</span> <span class=\"alg-name\">Reinforce</span>($\\pi_\\theta$, $\\alpha$)"
        );
    }

    #[test]
    fn comment_is_split_off_outside_math() {
        let s = mark_up("$x \\leftarrow a // b$ // integer division");
        assert!(
            s.markdown
                .ends_with("<span class=\"alg-comment\">\u{25B7} integer division</span>"),
            "got: {}",
            s.markdown
        );
        assert!(s.markdown.contains("$x \\leftarrow a // b$"));
        let url = mark_up("fetch https://example.com/data");
        assert!(!url.markdown.contains("alg-comment"));
    }

    #[test]
    fn angle_brackets_are_escaped_outside_math_and_code() {
        let s = mark_up("if a<b and c>d then");
        assert!(
            s.markdown.contains("a&lt;b and c&gt;d"),
            "got: {}",
            s.markdown
        );
        let math = mark_up("if $a<b$ then");
        assert!(math.markdown.contains("$a<b$"));
        let code = mark_up("run `a<b`");
        assert!(code.markdown.contains("`a<b`"));
    }

    #[test]
    fn preprocessed_anchors_pass_through() {
        let cite = r##"<sup><a href="#ref-k" class="cite">[1]</a></sup>"##;
        let s = mark_up(&format!("Adam step {cite}"));
        assert!(s.markdown.contains(cite), "got: {}", s.markdown);
        let figref = r##"see <a href="#fig:a">Figure 1</a>"##;
        assert!(mark_up(figref).markdown.contains(figref));
    }

    #[test]
    fn render_emits_caption_numbers_and_depths() {
        let block =
            parse("title: Sum\nlabel: alg:sum\n---\nInput: $n$\nfor $i$ do\n    step\n\nend for\n");
        let html = render(&block, 3, &mut identity);
        assert!(html.starts_with("<figure class=\"algorithm\" id=\"alg:sum\">"));
        assert!(html.contains(
            "<figcaption><span class=\"alg-number\">Algorithm 3</span> <span class=\"alg-title\">Sum</span></figcaption>"
        ));
        assert!(html.contains(
            "<li class=\"alg-head\"><span class=\"alg-stmt\"><span class=\"alg-kw\">Input:</span>"
        ));
        assert!(html.contains("<li style=\"--depth:1\"><span class=\"alg-stmt\">step</span></li>"));
        assert!(html.contains("<li class=\"alg-gap\" aria-hidden=\"true\"></li>"));
        assert!(html.ends_with("</ol>\n</figure>\n"));
    }

    #[test]
    fn render_without_title_or_label_still_numbers() {
        let html = render(&parse("return $x$\n"), 1, &mut identity);
        assert!(html.starts_with("<figure class=\"algorithm\">"));
        assert!(html.contains("<span class=\"alg-number\">Algorithm 1</span></figcaption>"));
    }

    #[test]
    fn numbering_is_sequential_and_first_label_wins() {
        let mut algs = Algorithms::default();
        assert_eq!(algs.assign(Some("a")), 1);
        assert_eq!(algs.assign(None), 2);
        assert_eq!(algs.assign(Some("a")), 3);
        assert_eq!(algs.labels().get("a"), Some(&1));
    }

    #[test]
    fn algref_links_and_flags_unknown_keys() {
        let mut labels = HashMap::new();
        labels.insert("alg:a".to_string(), 2u32);
        let out = replace_algrefs(r"<p>See \algref{alg:a} and \algref{nope}.</p>", &labels);
        assert_eq!(
            out,
            r##"<p>See <a href="#alg:a">Algorithm 2</a> and [?:nope].</p>"##
        );
    }

    #[test]
    fn algref_inside_code_stays_literal() {
        let mut labels = HashMap::new();
        labels.insert("k".to_string(), 1u32);
        let html = r"<p>Write <code>\algref{k}</code> to link \algref{k}.</p>";
        let out = replace_algrefs(html, &labels);
        assert!(out.contains(r"<code>\algref{k}</code>"), "got: {out}");
        assert!(
            out.contains(r##"link <a href="#k">Algorithm 1</a>."##),
            "got: {out}"
        );
    }
}
