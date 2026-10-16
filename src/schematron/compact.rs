//! Annotations of compact-syntax (`.rnc`) schemas, as XML.
//!
//! Compact-syntax annotations (`[ … ]` before a pattern or definition,
//! `>> x:y [ … ]` after a pattern, and `x:y [ … ]` between definitions) are
//! XML elements written another way. [`annotations`] rewrites the ones a
//! schema contains as an XML document, in schema order, with an `include` or
//! `externalRef` element in the RELAX NG namespace for each file the schema
//! references, so the XML-syntax extraction reads both syntaxes alike.
//!
//! Only as much of the compact syntax is parsed as is needed to tell
//! annotations apart; the schema has already been compiled, so its
//! patterns are known to be well-formed.

const RELAXNG_NS: &str = "http://relaxng.org/ns/structure/1.0";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Clone, Debug, PartialEq)]
enum Token {
    /// An identifier, keyword or prefixed name; `escaped` when written with
    /// a leading `\`, which makes a keyword an ordinary identifier.
    Name {
        text: String,
        escaped: bool,
    },
    Literal(String),
    /// `>>`, which introduces a follow annotation.
    Follow,
    Symbol(char),
}

/// The annotation elements of compact-syntax `text`, and the files it
/// includes or references, as an XML document.
pub(super) fn annotations(text: &str) -> Result<String, String> {
    let tokens = lex(text)?;
    let mut parser = Parser {
        tokens,
        position: 0,
        namespaces: vec![("xml".into(), XML_NS.into())],
        xml: String::new(),
    };
    parser.declarations()?;
    parser.body()?;
    let mut xml = String::from("<annotations");
    for (prefix, uri) in parser
        .namespaces
        .iter()
        .filter(|(prefix, _)| prefix != "xml")
    {
        xml.push_str(&format!(" xmlns:{prefix}=\"{}\"", escape(uri, true)));
    }
    xml.push('>');
    xml.push_str(&parser.xml);
    xml.push_str("</annotations>");
    Ok(xml)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    /// `xml`, and the prefixes declared with `namespace` or `default
    /// namespace`.
    namespaces: Vec<(String, String)>,
    xml: String,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }

    fn is_keyword(&self, offset: usize, keyword: &str) -> bool {
        matches!(self.peek_at(offset), Some(Token::Name { text, escaped: false }) if text == keyword)
    }

    fn is_symbol(&self, offset: usize, symbol: char) -> bool {
        self.peek_at(offset) == Some(&Token::Symbol(symbol))
    }

    fn expect_symbol(&mut self, symbol: char) -> Result<(), String> {
        if self.is_symbol(0, symbol) {
            self.position += 1;
            Ok(())
        } else {
            Err(format!("expected `{symbol}` in annotation"))
        }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Token::Name { text, .. }) => {
                let text = text.clone();
                self.position += 1;
                Ok(text)
            }
            _ => Err("expected a name in annotation".into()),
        }
    }

    /// A literal, joining `"a" ~ "b"` segments.
    fn literal(&mut self) -> Result<String, String> {
        let mut value = String::new();
        loop {
            match self.peek() {
                Some(Token::Literal(segment)) => value.push_str(segment),
                _ => return Err("expected a string literal".into()),
            }
            self.position += 1;
            if !self.is_symbol(0, '~') {
                return Ok(value);
            }
            self.position += 1;
        }
    }

    /// `namespace`, `default namespace` and `datatypes` declarations.
    fn declarations(&mut self) -> Result<(), String> {
        loop {
            let prefix = if self.is_keyword(0, "namespace") {
                self.position += 1;
                Some(self.name()?)
            } else if self.is_keyword(0, "default") && self.is_keyword(1, "namespace") {
                self.position += 2;
                if self.is_symbol(0, '=') {
                    None
                } else {
                    Some(self.name()?)
                }
            } else if self.is_keyword(0, "datatypes") {
                self.position += 1;
                self.name()?;
                None
            } else {
                return Ok(());
            };
            self.expect_symbol('=')?;
            let uri = if self.is_keyword(0, "inherit") {
                self.position += 1;
                None
            } else {
                Some(self.literal()?)
            };
            if let Some(prefix) = prefix {
                self.namespaces.retain(|(bound, _)| *bound != prefix);
                // A prefix bound to no namespace can't name annotations.
                if let Some(uri) = uri.filter(|uri| !uri.is_empty()) {
                    self.namespaces.push((prefix, uri));
                }
            }
        }
    }

    /// Scans the patterns and definitions for annotations, includes and
    /// external references.
    fn body(&mut self) -> Result<(), String> {
        // Whether the previous token ends a pattern or definition, so that a
        // name followed by `[` is an annotation element between definitions
        // rather than a datatype name followed by the next definition's
        // leading annotation. True at the start, where definitions begin.
        let mut after_pattern = true;
        while let Some(token) = self.peek() {
            match token {
                Token::Follow => {
                    self.position += 1;
                    let name = self.name()?;
                    self.element(name)?;
                    after_pattern = true;
                }
                Token::Name { .. } if after_pattern && self.is_symbol(1, '[') => {
                    let name = self.name()?;
                    self.element(name)?;
                }
                Token::Symbol('[') => {
                    self.leading_annotation()?;
                    after_pattern = false;
                }
                Token::Name {
                    text,
                    escaped: false,
                } if (text == "include" || text == "external")
                    && matches!(self.peek_at(1), Some(Token::Literal(_))) =>
                {
                    let element = if text == "include" {
                        "include"
                    } else {
                        "externalRef"
                    };
                    self.position += 1;
                    let href = self.literal()?;
                    self.xml.push_str(&format!(
                        "<{element} xmlns=\"{RELAXNG_NS}\" href=\"{}\"/>",
                        escape(&href, true)
                    ));
                    if self.is_keyword(0, "inherit") && self.is_symbol(1, '=') {
                        self.position += 3;
                    }
                    // An include's body holds definitions.
                    if element == "include" && self.is_symbol(0, '{') {
                        self.position += 1;
                    }
                    after_pattern = true;
                }
                Token::Symbol('{') => {
                    // A grammar's or div's body holds definitions; any other
                    // braces hold a pattern, name class or parameters.
                    after_pattern = matches!(
                        self.position.checked_sub(1).map(|i| &self.tokens[i]),
                        Some(Token::Name { text, escaped: false }) if text == "grammar" || text == "div"
                    );
                    self.position += 1;
                }
                Token::Name { .. }
                | Token::Literal(_)
                | Token::Symbol('}' | ')' | '?' | '*' | '+') => {
                    after_pattern = true;
                    self.position += 1;
                }
                Token::Symbol(_) => {
                    after_pattern = false;
                    self.position += 1;
                }
            }
        }
        Ok(())
    }

    /// `[ attribute* element* ]` before a pattern or definition: the
    /// attributes belong to the annotated RELAX NG element, and the elements
    /// are its annotations.
    fn leading_annotation(&mut self) -> Result<(), String> {
        self.expect_symbol('[')?;
        while matches!(self.peek(), Some(Token::Name { .. })) && self.is_symbol(1, '=') {
            self.position += 2;
            self.literal()?;
        }
        while !self.is_symbol(0, ']') {
            let name = self.name()?;
            self.element(name)?;
        }
        self.position += 1;
        Ok(())
    }

    /// An annotation element named `name`, whose `[` is the next token,
    /// written out as XML. Nested elements are kept on a stack, not
    /// parsed recursively.
    fn element(&mut self, name: String) -> Result<(), String> {
        let mut open = vec![self.start_tag(name)?];
        while let Some(name) = open.last() {
            match self.peek() {
                Some(Token::Symbol(']')) => {
                    self.position += 1;
                    self.xml.push_str(&format!("</{name}>"));
                    open.pop();
                }
                Some(Token::Literal(_)) => {
                    let text = self.literal()?;
                    self.xml.push_str(&escape(&text, false));
                }
                Some(Token::Name { .. }) => {
                    let name = self.name()?;
                    let tag = self.start_tag(name)?;
                    open.push(tag);
                }
                Some(_) => return Err("unexpected token in annotation".into()),
                None => return Err("unterminated annotation".into()),
            }
        }
        Ok(())
    }

    /// `[` and the attributes that follow it, written as the start tag of
    /// element `name`. Returns the name to close it with.
    fn start_tag(&mut self, name: String) -> Result<String, String> {
        self.check_name(&name)?;
        self.expect_symbol('[')?;
        self.xml.push('<');
        self.xml.push_str(&name);
        while matches!(self.peek(), Some(Token::Name { .. })) && self.is_symbol(1, '=') {
            let attribute = self.name()?;
            self.check_name(&attribute)?;
            self.position += 1;
            let value = self.literal()?;
            self.xml
                .push_str(&format!(" {attribute}=\"{}\"", escape(&value, true)));
        }
        self.xml.push('>');
        Ok(name)
    }

    /// Checks that `name` is an NCName, or a prefixed name whose prefix is
    /// declared.
    fn check_name(&self, name: &str) -> Result<(), String> {
        let (prefix, local) = match name.split_once(':') {
            Some((prefix, local)) => (Some(prefix), local),
            None => (None, name),
        };
        if !is_ncname(local) || prefix.is_some_and(|prefix| !is_ncname(prefix)) {
            return Err(format!("`{name}` is not a valid annotation name"));
        }
        match prefix {
            Some(prefix) if !self.namespaces.iter().any(|(bound, _)| bound == prefix) => {
                Err(format!("undeclared namespace prefix `{prefix}`"))
            }
            _ => Ok(()),
        }
    }
}

fn is_ncname(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.') || !c.is_ascii())
}

/// Escapes `text` for XML content, or for a quoted attribute value, keeping
/// carriage returns, and in attributes tabs and newlines, as written.
fn escape(text: &str, attribute: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if attribute => escaped.push_str("&quot;"),
            '\r' => escaped.push_str("&#xD;"),
            '\n' if attribute => escaped.push_str("&#xA;"),
            '\t' if attribute => escaped.push_str("&#x9;"),
            c => escaped.push(c),
        }
    }
    escaped
}

/// Splits compact-syntax text into tokens, dropping comments. `\x{N}`
/// escapes are decoded first; the characters they produce never delimit
/// anything, so an escaped quote or newline is part of a literal.
fn lex(text: &str) -> Result<Vec<Token>, String> {
    let chars = unescape(text)?;
    let mut tokens = Vec::new();
    let mut i = 0;
    // The unescaped character at `i`, if any.
    let plain = |i: usize| match chars.get(i) {
        Some(&(c, false)) => Some(c),
        _ => None,
    };
    while i < chars.len() {
        let (c, escaped) = chars[i];
        if escaped {
            // Starts a name, like any other name character.
        } else if c.is_whitespace() {
            i += 1;
            continue;
        } else if c == '#' {
            while i < chars.len() && !matches!(plain(i), Some('\n' | '\r')) {
                i += 1;
            }
            continue;
        } else if c == '"' || c == '\'' {
            let triple = plain(i + 1) == Some(c) && plain(i + 2) == Some(c);
            let delimiter = if triple { 3 } else { 1 };
            i += delimiter;
            let mut value = String::new();
            loop {
                match chars.get(i) {
                    None => return Err("unterminated string literal".into()),
                    Some(&(q, false))
                        if q == c && (1..delimiter).all(|offset| plain(i + offset) == Some(c)) =>
                    {
                        i += delimiter;
                        break;
                    }
                    Some(&('\n' | '\r', false)) if !triple => {
                        return Err("newline in a string literal".into());
                    }
                    Some(&(next, _)) => {
                        value.push(next);
                        i += 1;
                    }
                }
            }
            tokens.push(Token::Literal(value));
            continue;
        } else if c == '>' && plain(i + 1) == Some('>') {
            tokens.push(Token::Follow);
            i += 2;
            continue;
        } else if "{}()[],|&?*+-=~>".contains(c) {
            tokens.push(Token::Symbol(c));
            i += 1;
            continue;
        }
        let quoted = !escaped && c == '\\';
        if quoted {
            i += 1;
        }
        let mut name = String::new();
        while let Some(&(c, escaped)) = chars.get(i) {
            if !escaped && (c.is_whitespace() || "{}()[],|&?*+=~>\"'#\\".contains(c)) {
                break;
            }
            name.push(c);
            i += 1;
        }
        if name.is_empty() {
            return Err(format!("unexpected `{c}` in compact syntax"));
        }
        tokens.push(Token::Name {
            text: name,
            escaped: quoted,
        });
    }
    Ok(tokens)
}

/// Decodes `\x{N}` escapes (any number of `x`), marking each character with
/// whether it came from one.
fn unescape(text: &str) -> Result<Vec<(char, bool)>, String> {
    let mut chars = Vec::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            let after = rest[1..].trim_start_matches('x');
            if after.len() < rest.len() - 1 && after.starts_with('{') {
                let end = after
                    .find('}')
                    .ok_or("unterminated `\\x{` escape in compact syntax")?;
                let digits = &after[1..end];
                let decoded = (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()))
                    .then(|| u32::from_str_radix(digits, 16).ok())
                    .flatten()
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("invalid escape `\\x{}`", &after[..=end]))?;
                chars.push((decoded, true));
                rest = &after[end + 1..];
                continue;
            }
        }
        chars.push((c, false));
        rest = &rest[c.len_utf8()..];
    }
    Ok(chars)
}
