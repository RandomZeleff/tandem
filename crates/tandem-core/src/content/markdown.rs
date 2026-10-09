//! Project descriptions and changelogs (Markdown mixed with HTML) turned into HTML the
//! launcher can show as is: no scripts, styles, forms or frames, links made absolute.

use ammonia::{Builder, Url, UrlRelative};
use pulldown_cmark::{html, Options, Parser};

/// Relative links in descriptions point into Modrinth's site.
const SITE: &str = "https://modrinth.com/";

/// Renders Markdown (with inline HTML) to sanitized HTML.
pub fn to_safe_html(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut rendered = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut rendered, Parser::new_ext(markdown, options));
    sanitizer().clean(&replace_frames(&rendered)).to_string()
}

fn sanitizer() -> Builder<'static> {
    let mut builder = Builder::default();
    builder
        // Authors center their banners with `align`; `style` stays out.
        .add_generic_attributes(&["align"])
        .add_tags(&["details", "summary"])
        .add_allowed_classes("a", &["md-video"])
        .set_tag_attribute_value("img", "loading", "lazy")
        .set_tag_attribute_value("img", "decoding", "async");
    if let Ok(base) = Url::parse(SITE) {
        builder.url_relative(UrlRelative::RewriteWithBase(base));
    }
    builder
}

/// Embedded frames cannot be shown safely: a YouTube video becomes its thumbnail linking
/// to the video, any other frame a plain link to what it embedded.
fn replace_frames(html: &str) -> String {
    // ASCII lowercasing keeps byte offsets, so indices found here are valid in `html`.
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut rest = 0;
    while let Some(start) = lower[rest..].find("<iframe").map(|i| rest + i) {
        let Some(open_end) = lower[start..].find('>').map(|i| start + i + 1) else {
            break;
        };
        let end = lower[open_end..]
            .find("</iframe>")
            .map_or(open_end, |i| open_end + i + "</iframe>".len());
        out.push_str(&html[rest..start]);
        if let Some(src) = attribute(&html[start..open_end], "src") {
            if let Some(id) = youtube_id(src) {
                out.push_str(&format!(
                    r#"<a class="md-video" href="https://www.youtube.com/watch?v={id}"><img src="https://i.ytimg.com/vi/{id}/hqdefault.jpg" alt="Vidéo YouTube"></a>"#
                ));
            } else if src.starts_with("https://") && !src.contains(['"', '<', '>']) {
                out.push_str(&format!(r#"<p><a href="{src}">Contenu intégré</a></p>"#));
            }
        }
        rest = end;
    }
    out.push_str(&html[rest..]);
    out
}

/// Value of `name="…"` (or single-quoted) inside an opening tag.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name).map(|i| from + i) {
        from = i + name.len();
        let preceded = lower[..i].ends_with(|c: char| c.is_ascii_whitespace());
        let after = lower[from..].trim_start();
        if !preceded || !after.starts_with('=') {
            continue;
        }
        let value = after[1..].trim_start();
        let offset = tag.len() - value.len();
        let quote = value.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let len = value[1..].find(quote)?;
        return Some(&tag[offset + 1..offset + 1 + len]);
    }
    None
}

/// Video id of a YouTube embed URL (`youtube.com/embed/<id>`, also the no-cookie domain).
fn youtube_id(src: &str) -> Option<&str> {
    let host_and_path = src.split_once("//").map_or(src, |(_, rest)| rest);
    let (host, path) = host_and_path.split_once('/')?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host != "youtube.com" && host != "youtube-nocookie.com" {
        return None;
    }
    let id = path.strip_prefix("embed/")?;
    let id = &id[..id.find(['?', '/', '&', '#']).unwrap_or(id.len())];
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown() {
        let html = to_safe_html("# Title\n\nSome **bold** text and a [link](https://example.com).\n\n| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(
            html.contains(r#"<a href="https://example.com" rel="noopener noreferrer">link</a>"#)
        );
        assert!(html.contains("<table>"));
    }

    #[test]
    fn removes_scripts_styles_and_handlers() {
        let html = to_safe_html(
            r#"<script>alert(1)</script><p style="color:red" onclick="x()" align="center">Hi</p><style>p{}</style><img src="https://a.b/c.png" onerror="x()">"#,
        );
        assert!(!html.contains("script"));
        assert!(!html.contains("style"));
        assert!(!html.contains("onclick"));
        assert!(!html.contains("onerror"));
        assert!(html.contains(r#"<p align="center">Hi</p>"#));
        assert!(html.contains(r#"loading="lazy""#));
    }

    #[test]
    fn neutralizes_javascript_links() {
        let html = to_safe_html(r#"<a href="javascript:alert(1)">x</a> [y](javascript:alert(2))"#);
        assert!(!html.contains("javascript"));
    }

    #[test]
    fn makes_relative_links_absolute() {
        let html = to_safe_html("[Sodium](/mod/sodium)");
        assert!(html.contains(r#"href="https://modrinth.com/mod/sodium""#));
    }

    #[test]
    fn turns_youtube_frames_into_thumbnails() {
        let html = to_safe_html(
            r#"<iframe width="560" height="315" src="https://www.youtube-nocookie.com/embed/-HqUkTtxtoU?si=x" title="YouTube video player" allowfullscreen></iframe>

After"#,
        );
        assert!(!html.contains("iframe"));
        assert!(html.contains(r#"href="https://www.youtube.com/watch?v=-HqUkTtxtoU""#));
        assert!(html.contains(r#"src="https://i.ytimg.com/vi/-HqUkTtxtoU/hqdefault.jpg""#));
        assert!(html.contains(r#"class="md-video""#));
        assert!(html.contains("After"));
    }

    #[test]
    fn links_other_frames_and_drops_unsafe_ones() {
        let html = to_safe_html(
            r#"<IFRAME SRC='https://example.com/map'></IFRAME><iframe src="javascript:x()"></iframe>"#,
        );
        assert!(!html.to_lowercase().contains("iframe"));
        assert!(html.contains(r#"href="https://example.com/map""#));
        assert!(!html.contains("javascript"));
    }

    #[test]
    fn keeps_classes_only_where_allowed() {
        let html = to_safe_html(
            r#"<a class="md-video evil" href="https://a.b">v</a><p class="evil">p</p>"#,
        );
        assert!(html.contains(r#"class="md-video""#));
        assert!(!html.contains("evil"));
    }

    #[test]
    fn reads_attributes() {
        assert_eq!(
            attribute(r#"<iframe data-src="no" src="yes">"#, "src"),
            Some("yes")
        );
        assert_eq!(attribute("<iframe src='single'>", "src"), Some("single"));
        assert_eq!(attribute("<iframe src=bare>", "src"), None);
        assert_eq!(
            youtube_id("https://youtube.com/embed/abc_D-1"),
            Some("abc_D-1")
        );
        assert_eq!(youtube_id("https://evil.com/embed/abc"), None);
        assert_eq!(youtube_id("https://www.youtube.com/watch?v=abc"), None);
    }
}
