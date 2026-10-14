//! Just enough SNBT for FTB Quests: find quoted strings with the key they belong to, and
//! replace them in place without touching the rest of the file; or read a whole file as
//! a tree (quest progress, see `journal`).

/// A quoted string of an SNBT document, and the key it is the value of (for strings in
/// a list, the list's key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnbtString {
    pub key: String,
    /// Position among the values of a list, `None` outside lists.
    pub index: Option<usize>,
    /// Byte range of the literal, quotes included.
    pub start: usize,
    pub end: usize,
    pub value: String,
}

#[derive(Debug)]
enum Frame {
    Compound,
    List { key: String, count: usize },
}

/// Every quoted value string, in document order. Keys themselves are not returned.
pub fn strings(text: &str) -> Vec<SnbtString> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut key = String::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                let Some((value, end)) = read_quoted(text, i) else {
                    break;
                };
                if next_non_space(bytes, end) == Some(b':') {
                    key = value;
                } else {
                    let (owner, index) = match stack.last_mut() {
                        Some(Frame::List { key, count }) => {
                            *count += 1;
                            (key.clone(), Some(*count - 1))
                        }
                        _ => (key.clone(), None),
                    };
                    out.push(SnbtString {
                        key: owner,
                        index,
                        start: i,
                        end,
                        value,
                    });
                }
                i = end;
            }
            b'{' => {
                stack.push(Frame::Compound);
                i += 1;
            }
            b'[' => {
                stack.push(Frame::List {
                    key: key.clone(),
                    count: 0,
                });
                i += 1;
            }
            b'}' | b']' => {
                stack.pop();
                i += 1;
            }
            c if is_bare(c) => {
                let start = i;
                while i < bytes.len() && is_bare(bytes[i]) {
                    i += 1;
                }
                if next_non_space(bytes, i) == Some(b':') {
                    key = text[start..i].to_owned();
                } else if let Some(Frame::List { count, .. }) = stack.last_mut() {
                    // Unquoted list values (numbers) still take a position.
                    *count += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn is_bare(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'+')
}

fn next_non_space(bytes: &[u8], mut i: usize) -> Option<u8> {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    bytes.get(i).copied()
}

/// The unescaped content of the literal starting at `start`, and the index after it.
fn read_quoted(text: &str, start: usize) -> Option<(String, usize)> {
    let quote = text[start..].chars().next()?;
    let mut value = String::new();
    let mut chars = text[start + 1..].char_indices();
    while let Some((offset, c)) = chars.next() {
        match c {
            '\\' => {
                let (_, escaped) = chars.next()?;
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    other => value.push(other),
                }
            }
            c if c == quote => return Some((value, start + 1 + offset + 1)),
            c => value.push(c),
        }
    }
    None
}

/// A double-quoted SNBT literal.
pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `text` with some literals replaced, given as `(start, end, new value)`.
pub fn replace(text: &str, mut edits: Vec<(usize, usize, String)>) -> String {
    edits.sort_by_key(|e| e.0);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end, value) in edits {
        if start < at {
            continue;
        }
        out.push_str(&text[at..start]);
        out.push_str(&quote(&value));
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

/// A flat compound of strings and lists of strings, the format of FTB Quests language
/// files (`config/ftbquests/quests/lang/<locale>.snbt`).
pub fn write_compound(entries: &[(String, Vec<String>)]) -> String {
    let mut out = String::from("{\n");
    for (key, values) in entries {
        out.push('\t');
        out.push_str(&quote(key));
        out.push_str(": ");
        if values.len() == 1 && !key.ends_with("_desc") {
            out.push_str(&quote(&values[0]));
        } else {
            out.push_str("[\n");
            for value in values {
                out.push_str("\t\t");
                out.push_str(&quote(value));
                out.push('\n');
            }
            out.push_str("\t]");
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

/// A parsed SNBT value. Numbers, booleans and other bare words stay as written.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Compound(Vec<(String, Value)>),
    List(Vec<Value>),
    Str(String),
    Bare(String),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) | Value::Bare(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Value] {
        match self {
            Value::List(items) => items,
            _ => &[],
        }
    }

    /// Keys of a compound.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        let entries: &[(String, Value)] = match self {
            Value::Compound(entries) => entries,
            _ => &[],
        };
        entries.iter().map(|(k, _)| k.as_str())
    }
}

/// The whole document as a tree (FTB Quests files: newline-separated, commas optional).
pub fn parse(text: &str) -> Option<Value> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        at: 0,
    };
    parser.value()
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn skip(&mut self) {
        while self.at < self.bytes.len()
            && (self.bytes[self.at].is_ascii_whitespace() || self.bytes[self.at] == b',')
        {
            self.at += 1;
        }
    }

    fn word(&mut self) -> Option<String> {
        let start = self.at;
        while self.at < self.bytes.len() && is_bare(self.bytes[self.at]) {
            self.at += 1;
        }
        (self.at > start).then(|| self.text[start..self.at].to_owned())
    }

    fn string(&mut self) -> Option<String> {
        let (value, end) = read_quoted(self.text, self.at)?;
        self.at = end;
        Some(value)
    }

    fn value(&mut self) -> Option<Value> {
        self.skip();
        match *self.bytes.get(self.at)? {
            b'{' => {
                self.at += 1;
                let mut entries = Vec::new();
                loop {
                    self.skip();
                    match *self.bytes.get(self.at)? {
                        b'}' => {
                            self.at += 1;
                            return Some(Value::Compound(entries));
                        }
                        b'"' | b'\'' => {
                            let key = self.string()?;
                            entries.push((key, self.entry_value()?));
                        }
                        _ => {
                            let key = self.word()?;
                            entries.push((key, self.entry_value()?));
                        }
                    }
                }
            }
            b'[' => {
                self.at += 1;
                // Typed arrays: `[I; 1, 2]`.
                let rest = &self.bytes[self.at..];
                if rest.len() >= 2 && rest[0].is_ascii_alphabetic() && rest[1] == b';' {
                    self.at += 2;
                }
                let mut items = Vec::new();
                loop {
                    self.skip();
                    if *self.bytes.get(self.at)? == b']' {
                        self.at += 1;
                        return Some(Value::List(items));
                    }
                    items.push(self.value()?);
                }
            }
            b'"' | b'\'' => self.string().map(Value::Str),
            _ => self.word().map(Value::Bare),
        }
    }

    fn entry_value(&mut self) -> Option<Value> {
        self.skip();
        if self.bytes.get(self.at) != Some(&b':') {
            return None;
        }
        self.at += 1;
        self.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAPTER: &str = r#"{
	filename: "archon"
	quests: [
		{
			title: "First \"Magic\" Steps"
			description: ["Line one", "", 'Line \'two\'']
			x: -2.5d
			tasks: [{ id: "1A", item: "minecraft:stick", type: "item" }]
		}
	]
	"quoted.key": "value"
}"#;

    #[test]
    fn finds_strings_with_their_keys() {
        let found = strings(CHAPTER);
        let pairs: Vec<(&str, Option<usize>, &str)> = found
            .iter()
            .map(|s| (s.key.as_str(), s.index, s.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("filename", None, "archon"),
                ("title", None, "First \"Magic\" Steps"),
                ("description", Some(0), "Line one"),
                ("description", Some(1), ""),
                ("description", Some(2), "Line 'two'"),
                ("id", None, "1A"),
                ("item", None, "minecraft:stick"),
                ("type", None, "item"),
                ("quoted.key", None, "value"),
            ]
        );
    }

    #[test]
    fn replaces_in_place() {
        let found = strings(CHAPTER);
        let title = found.iter().find(|s| s.key == "title").unwrap();
        let out = replace(
            CHAPTER,
            vec![(
                title.start,
                title.end,
                "Premiers pas \"magiques\"".to_owned(),
            )],
        );
        assert!(out.contains(r#"title: "Premiers pas \"magiques\"""#));
        assert!(out.contains("x: -2.5d"));
        assert_eq!(strings(&out).len(), found.len());
    }

    #[test]
    fn writes_language_files() {
        let text = write_compound(&[
            ("chapter.1.title".to_owned(), vec!["Début".to_owned()]),
            (
                "quest.2.quest_desc".to_owned(),
                vec!["a".to_owned(), "b\nc".to_owned()],
            ),
        ]);
        let found = strings(&text);
        assert_eq!(found[0].key, "chapter.1.title");
        assert_eq!(found[2].value, "b\nc");
        assert_eq!(found[2].index, Some(1));
    }
}
