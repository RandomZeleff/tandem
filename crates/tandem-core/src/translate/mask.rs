//! Codes the game interprets inside texts (`%s`, `§c`, `{0}`, `$(item)`, line breaks…)
//! are swapped for numbered tokens before a text goes to the model, then put back. A
//! translation that loses, duplicates or invents a token is rejected.

/// Opening and closing marks of a token: rare characters models copy faithfully.
const OPEN: char = '⟦';
const CLOSE: char = '⟧';

/// A text ready for the model, and the codes its tokens stand for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Masked {
    pub text: String,
    pub codes: Vec<String>,
}

/// Replaces every code in `text` by `⟦n⟧`.
pub fn mask(text: &str) -> Masked {
    mask_with(text, false, false)
}

/// Same as [`mask`], numbers included: texts that differ only by their numbers
/// ("Pattern 3", "Pattern 12") then share one masked form and one translation.
pub fn mask_numbers(text: &str) -> Masked {
    mask_with(text, false, true)
}

/// Same as [`mask`] for a line of HTML: tags and entities are codes too.
pub fn mask_html(text: &str) -> Masked {
    mask_with(text, true, false)
}

fn mask_with(text: &str, html: bool, numbers: bool) -> Masked {
    let mut out = String::with_capacity(text.len());
    let mut codes = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let number = || {
            let run = chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
            (numbers && run > 0 && (i == 0 || !chars[i - 1].is_alphanumeric())).then_some(run)
        };
        if let Some(len) = html
            .then(|| html_code_at(&chars, i))
            .flatten()
            .or_else(|| code_at(&chars, i))
            .or_else(number)
        {
            let code: String = chars[i..i + len].iter().collect();
            out.push_str(&format!("{OPEN}{}{CLOSE}", codes.len()));
            codes.push(code);
            i += len;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    Masked { text: out, codes }
}

/// Tags (`<a href="…">`) and entities (`&amp;`, `&#39;`).
fn html_code_at(chars: &[char], i: usize) -> Option<usize> {
    match chars[i] {
        '<' => chars[i..].iter().position(|&c| c == '>').map(|end| end + 1),
        '&' => {
            let end = chars[i + 1..].iter().take(10).position(|&c| c == ';')?;
            let name = &chars[i + 1..i + 1 + end];
            (!name.is_empty() && name.iter().all(|c| c.is_ascii_alphanumeric() || *c == '#'))
                .then_some(end + 2)
        }
        _ => None,
    }
}

/// Length in chars of the code starting at `i`, if any.
fn code_at(chars: &[char], i: usize) -> Option<usize> {
    let c = chars[i];
    let next = chars.get(i + 1).copied();
    match c {
        '\n' => Some(1),
        // Formatting codes: `§a`, and `&a` as used by quest and config mods.
        '§' => next.map(|_| 2),
        '&' if next.is_some_and(is_format_char) && (i == 0 || !chars[i - 1].is_alphanumeric()) => {
            Some(2)
        }
        '%' => printf_len(&chars[i..]),
        '{' => {
            // `{0}`, `{name}`, `{image:…}`, `{@pagebreak}`: anything without nested braces.
            let end = chars[i + 1..]
                .iter()
                .take(200)
                .position(|&c| c == '}' || c == '{')?;
            (chars[i + 1 + end] == '}').then_some(end + 2)
        }
        // Patchouli macros: `$(item)`, `$(l:path)`, `$()`, `$(br)`.
        '$' if next == Some('(') => {
            let end = chars[i + 2..].iter().take(200).position(|&c| c == ')')?;
            Some(end + 3)
        }
        _ => None,
    }
}

fn is_format_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c.to_ascii_lowercase(), 'a'..='f' | 'k'..='o' | 'r')
}

/// `%s`, `%d`, `%1$s`, `%.2f`, `%%`.
fn printf_len(chars: &[char]) -> Option<usize> {
    let mut i = 1;
    if chars.get(1) == Some(&'%') {
        return Some(2);
    }
    // Optional `n$` argument index.
    let digits = chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && chars.get(i + digits) == Some(&'$') {
        i += digits + 1;
    }
    i += chars[i..]
        .iter()
        .take_while(|c| matches!(c, '-' | '+' | '0' | '#' | ','))
        .count();
    i += chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    if chars.get(i) == Some(&'.') {
        i += 1;
        i += chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    }
    match chars.get(i) {
        Some(c) if "sdfxXcbeEgGhHon".contains(*c) => Some(i + 1),
        _ => None,
    }
}

/// Puts the codes back into a translated text. `None` when tokens are missing,
/// repeated or unknown: the translation cannot be trusted.
pub fn unmask(translated: &str, codes: &[String]) -> Option<String> {
    let mut out = String::with_capacity(translated.len());
    let mut seen = vec![false; codes.len()];
    let mut rest = translated;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + OPEN.len_utf8()..];
        let end = after.find(CLOSE)?;
        let index: usize = after[..end].trim().parse().ok()?;
        if index >= codes.len() || seen[index] {
            return None;
        }
        seen[index] = true;
        out.push_str(&codes[index]);
        rest = &after[end + CLOSE.len_utf8()..];
    }
    if rest.contains(CLOSE) {
        return None;
    }
    out.push_str(rest);
    seen.iter().all(|s| *s).then_some(out)
}

/// Whether a text is worth translating: it has words, not just codes and numbers.
pub fn has_words(text: &str) -> bool {
    let masked = mask(text).text;
    let mut letters = 0;
    let mut in_token = false;
    for c in masked.chars() {
        match c {
            OPEN => in_token = true,
            CLOSE => in_token = false,
            c if !in_token && c.is_alphabetic() => {
                letters += 1;
                if letters >= 2 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(text: &str) -> Masked {
        let masked = mask(text);
        assert_eq!(unmask(&masked.text, &masked.codes).as_deref(), Some(text));
        masked
    }

    #[test]
    fn masks_codes() {
        let m = roundtrip(
            "§6Gold§r: %s of %1$d (%.1f%%)\nNext {0} $(item)apple$() {image:a:b.png width:50}",
        );
        assert_eq!(
            m.text,
            "⟦0⟧Gold⟦1⟧: ⟦2⟧ of ⟦3⟧ (⟦4⟧⟦5⟧)⟦6⟧Next ⟦7⟧ ⟦8⟧apple⟦9⟧ ⟦10⟧"
        );
        assert_eq!(m.codes[3], "%1$d");
        assert_eq!(m.codes[10], "{image:a:b.png width:50}");
    }

    #[test]
    fn leaves_plain_text() {
        assert_eq!(
            roundtrip("100% sure, R&D team & friends {unclosed")
                .codes
                .len(),
            0
        );
        assert_eq!(roundtrip("&eYellow &r and Tom&co").codes, ["&e", "&r"]);
    }

    #[test]
    fn rejects_broken_translations() {
        let codes = vec!["%s".to_owned(), "§c".to_owned()];
        assert_eq!(
            unmask("⟦1⟧Danger ⟦0⟧", &codes).as_deref(),
            Some("§cDanger %s")
        );
        assert_eq!(unmask("Danger ⟦0⟧", &codes), None);
        assert_eq!(unmask("⟦0⟧ ⟦0⟧ ⟦1⟧", &codes), None);
        assert_eq!(unmask("⟦0⟧ ⟦1⟧ ⟦2⟧", &codes), None);
        assert_eq!(unmask("⟦0⟧ ⟦1⟧⟧", &codes), None);
        assert_eq!(unmask("⟦ 0 ⟧⟦1⟧", &codes).as_deref(), Some("%s§c"));
    }

    #[test]
    fn masks_numbers_for_variants() {
        let a = mask_numbers("Pattern 3 (x12)");
        let b = mask_numbers("Pattern 15 (x12)");
        assert_eq!(a.text, b.text);
        assert_eq!(a.text, "Pattern ⟦0⟧ (x12)");
        assert_eq!(
            unmask("Motif ⟦0⟧ (x12)", &b.codes).as_deref(),
            Some("Motif 15 (x12)")
        );
        assert_eq!(mask_numbers("Tier 2 %s").codes, ["2", "%s"]);
    }

    #[test]
    fn masks_html() {
        let m = mask_html("<p>Use <strong>Tom &amp; Co</strong> %s</p>");
        assert_eq!(m.text, "⟦0⟧Use ⟦1⟧Tom ⟦2⟧ Co⟦3⟧ ⟦4⟧⟦5⟧");
        assert_eq!(
            unmask(&m.text, &m.codes).as_deref(),
            Some("<p>Use <strong>Tom &amp; Co</strong> %s</p>")
        );
    }

    #[test]
    fn detects_words() {
        assert!(has_words("Iron Ingot"));
        assert!(!has_words("%s / %s"));
        assert!(!has_words("§a42"));
        assert!(!has_words("{0}"));
        assert!(has_words("x%sy is ok"));
    }
}
