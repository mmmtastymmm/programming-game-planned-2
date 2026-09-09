//! Tokens with significant indentation (`docs/01-language/syntax.md`,
//! Lexical rules). Indentation is spaces; a tab in indentation is a parse
//! error; newlines inside brackets are whitespace; a backslash joins lines.

use crate::errors::LoadError;
use crate::num::Num;

/// The reserved words: Python's that the language keeps, plus the ones it
/// reserves so that using them is a parse error rather than a call to a
/// missing name.
pub const KEYWORDS: &[&str] = &[
    "and", "as", "break", "class", "continue", "def", "elif", "else", "except", "False", "finally",
    "for", "from", "if", "import", "in", "is", "lambda", "match", "case", "None", "not", "or",
    "pass", "raise", "return", "True", "try", "while", // reserved, not statements:
    "del", "assert", "with", "async", "await", "yield", "global", "nonlocal",
];

/// Multi-character operators, longest first so the lexer matches greedily.
const OPS: &[&str] = &[
    "**=", "//=", "**", "//", "<=", ">=", "==", "!=", "+=", "-=", "*=", "/=", "%=", "|=", "&=",
    "^=", "+", "-", "*", "/", "%", "<", ">", "=", "(", ")", "[", "]", "{", "}", ",", ":", ".", ";",
    "|", "&", "^",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Indent,
    Dedent,
    Newline,
    Name(String),
    Keyword(&'static str),
    Num(Num),
    /// A plain string literal, escapes processed.
    Str(String),
    /// An f-string's raw body, escapes unprocessed; the parser splits it.
    FStr(String),
    Op(&'static str),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
}

pub struct Lexer<'a> {
    file: &'a str,
    src: Vec<char>,
    pos: usize,
    line: u32,
    indents: Vec<usize>,
    depth: u32,
    at_line_start: bool,
    out: Vec<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a str, src: &str) -> Lexer<'a> {
        Lexer {
            file,
            src: src.chars().collect(),
            pos: 0,
            line: 1,
            indents: vec![0],
            depth: 0,
            at_line_start: true,
            out: Vec::new(),
        }
    }

    fn err(&self, message: impl Into<String>) -> LoadError {
        LoadError {
            file: self.file.to_string(),
            line: self.line,
            message: message.into(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, k: usize) -> Option<char> {
        self.src.get(self.pos.wrapping_add(k)).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos = self.pos.wrapping_add(1);
        }
        c
    }

    fn push(&mut self, tok: Tok) {
        self.out.push(Token {
            tok,
            line: self.line,
        });
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LoadError> {
        loop {
            if self.at_line_start && self.depth == 0 {
                self.handle_indentation()?;
                self.at_line_start = false;
                if self.pos >= self.src.len() {
                    break;
                }
            }
            let Some(c) = self.peek() else { break };
            match c {
                ' ' => {
                    self.bump();
                }
                '\t' => return Err(self.err("a tab character is a parse error")),
                '#' => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                '\n' => {
                    self.bump();
                    if self.depth == 0 {
                        if !matches!(
                            self.out.last(),
                            Some(Token {
                                tok: Tok::Newline | Tok::Indent | Tok::Dedent,
                                ..
                            }) | None
                        ) {
                            self.push(Tok::Newline);
                        }
                        self.at_line_start = true;
                    }
                    self.line = self.line.wrapping_add(1);
                }
                '\r' => {
                    self.bump();
                }
                '\\' => {
                    if self.peek_at(1) == Some('\n') {
                        self.bump();
                        self.bump();
                        self.line = self.line.wrapping_add(1);
                    } else {
                        return Err(
                            self.err("a backslash outside a string joins lines and nothing else")
                        );
                    }
                }
                c if c.is_ascii_digit()
                    || (c == '.' && self.peek_at(1).is_some_and(|d| d.is_ascii_digit())) =>
                {
                    self.number()?;
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    self.name_or_string()?;
                }
                '\'' | '"' => {
                    self.string(false, false)?;
                }
                _ => self.operator()?,
            }
        }
        if self.depth != 0 {
            return Err(self.err("a bracket is never closed"));
        }
        if !matches!(
            self.out.last(),
            Some(Token {
                tok: Tok::Newline | Tok::Dedent,
                ..
            }) | None
        ) {
            self.push(Tok::Newline);
        }
        while self.indents.len() > 1 {
            self.indents.pop();
            self.push(Tok::Dedent);
        }
        self.push(Tok::Eof);
        Ok(self.out)
    }

    fn handle_indentation(&mut self) -> Result<(), LoadError> {
        // Measure the indentation of the next non-blank line; blank and
        // comment-only lines are skipped entirely.
        loop {
            let mut width: usize = 0;
            let mut i = self.pos;
            while let Some(&c) = self.src.get(i) {
                match c {
                    ' ' => {
                        width = width.wrapping_add(1);
                        i = i.wrapping_add(1);
                    }
                    '\t' => return Err(self.err("a tab character in indentation is a parse error")),
                    _ => break,
                }
            }
            match self.src.get(i) {
                None => {
                    self.pos = i;
                    return Ok(());
                }
                Some('\n') => {
                    self.pos = i.wrapping_add(1);
                    self.line = self.line.wrapping_add(1);
                    continue;
                }
                Some('\r') => {
                    self.pos = i.wrapping_add(1);
                    continue;
                }
                Some('#') => {
                    let mut j = i;
                    while let Some(&c) = self.src.get(j) {
                        if c == '\n' {
                            break;
                        }
                        j = j.wrapping_add(1);
                    }
                    self.pos = j;
                    continue;
                }
                Some(_) => {
                    self.pos = i;
                    let current = *self.indents.last().unwrap_or(&0);
                    if width > current {
                        self.indents.push(width);
                        self.push(Tok::Indent);
                    } else {
                        while width < *self.indents.last().unwrap_or(&0) {
                            self.indents.pop();
                            self.push(Tok::Dedent);
                        }
                        if width != *self.indents.last().unwrap_or(&0) {
                            return Err(self.err("indentation does not match any enclosing block"));
                        }
                    }
                    return Ok(());
                }
            }
        }
    }

    fn number(&mut self) -> Result<(), LoadError> {
        let start = self.pos;
        if self.peek() == Some('0')
            && matches!(self.peek_at(1), Some('x' | 'X' | 'o' | 'O' | 'b' | 'B'))
        {
            return Err(self.err("hex, octal and binary literals do not exist"));
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
            self.bump();
        }
        if self.peek() == Some('.') {
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err(self.err("an exponent needs digits"));
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            return Err(self.err("a number runs into a name"));
        }
        let text: String = self.src[start..self.pos].iter().collect();
        let n = Num::parse_literal(&text)
            .ok_or_else(|| self.err(format!("`{text}` is not a representable num literal")))?;
        self.push(Tok::Num(n));
        Ok(())
    }

    fn name_or_string(&mut self) -> Result<(), LoadError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        let word: String = self.src[start..self.pos].iter().collect();
        // A string prefix: r, f, rf, fr, in any case.
        if matches!(self.peek(), Some('\'' | '"')) {
            let lower = word.to_ascii_lowercase();
            let (raw, fstr) = match lower.as_str() {
                "r" => (true, false),
                "f" => (false, true),
                "rf" | "fr" => (true, true),
                _ => return Err(self.err(format!("`{word}` is not a string prefix"))),
            };
            return self.string(raw, fstr);
        }
        if let Some(kw) = KEYWORDS.iter().find(|k| **k == word) {
            self.push(Tok::Keyword(kw));
        } else {
            if !word.is_ascii() {
                return Err(self.err("identifiers are ASCII"));
            }
            self.push(Tok::Name(word));
        }
        Ok(())
    }

    fn string(&mut self, raw: bool, fstr: bool) -> Result<(), LoadError> {
        let quote = self.bump().unwrap_or('"');
        let triple = self.peek() == Some(quote) && self.peek_at(1) == Some(quote);
        if triple {
            self.bump();
            self.bump();
        }
        let mut body = String::new();
        loop {
            let Some(c) = self.bump() else {
                return Err(self.err("a string is never closed"));
            };
            if c == quote {
                if !triple {
                    break;
                }
                if self.peek() == Some(quote) && self.peek_at(1) == Some(quote) {
                    self.bump();
                    self.bump();
                    break;
                }
                body.push(c);
                continue;
            }
            if c == '\n' {
                if !triple {
                    return Err(self.err("a string is never closed"));
                }
                self.line = self.line.wrapping_add(1);
                body.push(c);
                continue;
            }
            if c == '\\' && !raw {
                if fstr {
                    // Keep escapes raw inside an f-string; the parser processes
                    // the literal segments after splitting on braces.
                    body.push('\\');
                    if let Some(n) = self.bump() {
                        body.push(n);
                    }
                    continue;
                }
                body.push(self.escape()?);
                continue;
            }
            body.push(c);
        }
        self.push(if fstr {
            Tok::FStr(body)
        } else {
            Tok::Str(body)
        });
        Ok(())
    }

    fn escape(&mut self) -> Result<char, LoadError> {
        let Some(c) = self.bump() else {
            return Err(self.err("a string is never closed"));
        };
        escape_char(c, |k| {
            let mut hex = String::new();
            for _ in 0..k {
                hex.push(
                    self.bump()
                        .ok_or_else(|| self.err("a string is never closed"))?,
                );
            }
            Ok(hex)
        })
        .map_err(|m| self.err(m))
    }

    fn operator(&mut self) -> Result<(), LoadError> {
        // Not in the language (`syntax.md`, "Not in the language"): the
        // walrus, `@`, `~`, shifts. Each is one token in Python and none is
        // ours, so they are refused as a unit rather than split.
        for banned in ["<<", ">>", ":=", "->", "@", "~"] {
            let slice: String = self.src
                [self.pos..self.src.len().min(self.pos.wrapping_add(banned.len()))]
                .iter()
                .collect();
            if slice == banned {
                return Err(self.err(format!("`{banned}` is not part of the language")));
            }
        }
        for op in OPS {
            let n = op.len();
            let slice: String = self.src[self.pos..self.src.len().min(self.pos.wrapping_add(n))]
                .iter()
                .collect();
            if slice == *op {
                self.pos = self.pos.wrapping_add(n);
                match *op {
                    "(" | "[" | "{" => self.depth = self.depth.wrapping_add(1),
                    ")" | "]" | "}" => {
                        if self.depth == 0 {
                            return Err(self.err(format!("`{op}` closes nothing")));
                        }
                        self.depth = self.depth.wrapping_sub(1);
                    }
                    _ => {}
                }
                self.push(Tok::Op(op));
                return Ok(());
            }
        }
        let c = self.peek().unwrap_or(' ');
        Err(self.err(format!("`{c}` is not part of the language")))
    }
}

/// Process one escape after a backslash. `hex(k)` reads `k` hex digits.
pub fn escape_char(
    c: char,
    mut hex: impl FnMut(usize) -> Result<String, LoadError>,
) -> Result<char, String> {
    Ok(match c {
        '\\' => '\\',
        '\'' => '\'',
        '"' => '"',
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        'f' => '\x0c',
        'v' => '\x0b',
        '0' => '\0',
        'x' | 'u' | 'U' => {
            let k = match c {
                'x' => 2,
                'u' => 4,
                _ => 8,
            };
            let digits = hex(k).map_err(|e| e.message)?;
            let v = u32::from_str_radix(&digits, 16)
                .map_err(|_| format!("`\\{c}` needs {k} hex digits"))?;
            char::from_u32(v).ok_or_else(|| format!("`\\{c}{digits}` is not a scalar value"))?
        }
        other => return Err(format!("`\\{other}` is not an escape")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        Lexer::new("t.py", src)
            .tokenize()
            .expect("lexes")
            .into_iter()
            .map(|t| t.tok)
            .collect()
    }

    #[test]
    fn indentation_becomes_tokens() {
        let t = toks("if x:\n    y = 1\n\n    z = 2\nw = 3\n");
        assert_eq!(
            t,
            vec![
                Tok::Keyword("if"),
                Tok::Name("x".into()),
                Tok::Op(":"),
                Tok::Newline,
                Tok::Indent,
                Tok::Name("y".into()),
                Tok::Op("="),
                Tok::Num(Num::ONE),
                Tok::Newline,
                Tok::Name("z".into()),
                Tok::Op("="),
                Tok::Num(Num::from_int(2).unwrap()),
                Tok::Newline,
                Tok::Dedent,
                Tok::Name("w".into()),
                Tok::Op("="),
                Tok::Num(Num::from_int(3).unwrap()),
                Tok::Newline,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn brackets_suspend_newlines_and_a_tab_is_an_error() {
        let t = toks("xs = [\n  1,\n  2,\n]\n");
        assert!(!t[..t.len().wrapping_sub(2)].contains(&Tok::Newline));
        assert!(Lexer::new("t.py", "if x:\n\ty = 1\n").tokenize().is_err());
        assert!(
            Lexer::new("t.py", "if x:\n   y = 1\n  z = 2\n")
                .tokenize()
                .is_err()
        );
    }

    #[test]
    fn strings_and_escapes() {
        assert_eq!(toks(r#""a\nb""#)[0], Tok::Str("a\nb".into()));
        assert_eq!(toks(r#"r"a\nb""#)[0], Tok::Str("a\\nb".into()));
        assert_eq!(toks("'''x\ny'''")[0], Tok::Str("x\ny".into()));
        assert_eq!(toks(r#"f"{x}!""#)[0], Tok::FStr("{x}!".into()));
        assert_eq!(toks(r#""\x41\u00e9""#)[0], Tok::Str("Aé".into()));
        assert!(Lexer::new("t.py", r#""\q""#).tokenize().is_err());
        assert!(Lexer::new("t.py", "b'x'").tokenize().is_err());
    }

    #[test]
    fn numbers_and_reserved_words() {
        assert_eq!(
            toks("1_000.5e1")[0],
            Tok::Num(Num::parse_literal("10005").unwrap())
        );
        assert!(Lexer::new("t.py", "0x1F").tokenize().is_err());
        assert!(Lexer::new("t.py", "1e-13").tokenize().is_err());
        assert_eq!(toks("del x")[0], Tok::Keyword("del"));
        assert!(Lexer::new("t.py", "x := 1").tokenize().is_err());
        assert!(Lexer::new("t.py", "a @ b").tokenize().is_err());
    }
}
