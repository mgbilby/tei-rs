//! XPath 2.0 regular expressions (XML Schema regex syntax plus `^`, `$`,
//! non-greedy quantifiers and flags), translated to the `regex` crate's
//! syntax. Back-references and Unicode block escapes (`\p{IsBasicLatin}`)
//! have no equivalent there and are rejected as unsupported; expressions
//! that aren't valid XPath regular expressions at all are told apart, as
//! XPath makes using them a dynamic error.

use std::fmt;

/// Why a regular expression can't be compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegexError {
    /// Not a valid XPath regular expression, with the error code using it
    /// raises: `FORX0001` for bad flags, `FORX0002` for a bad expression.
    Invalid(&'static str, String),
    /// Valid, but beyond what can be translated for the `regex` crate.
    Unsupported(String),
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(_, message) | Self::Unsupported(message) => f.write_str(message),
        }
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, RegexError> {
    Err(RegexError::Invalid("FORX0002", message.into()))
}

pub(crate) struct CompiledRegex {
    pub regex: regex::Regex,
    /// Whether the expression matches the empty string, which `replace()`
    /// and `tokenize()` must reject.
    pub matches_empty: bool,
}

impl fmt::Debug for CompiledRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}/", self.regex.as_str())
    }
}

impl CompiledRegex {
    pub(crate) fn new(pattern: &str, flags: &str) -> Result<Self, RegexError> {
        let mut dot_all = false;
        let mut prefix = String::new();
        let mut extended = false;
        let mut literal = false;
        for flag in flags.chars() {
            match flag {
                's' => dot_all = true,
                'm' => prefix.push('m'),
                'i' => prefix.push('i'),
                'x' => extended = true,
                'q' => literal = true,
                other => {
                    return Err(RegexError::Invalid(
                        "FORX0001",
                        format!("invalid flag `{other}`"),
                    ));
                }
            }
        }
        let translated = if literal {
            regex::escape(pattern)
        } else {
            translate(pattern, dot_all, extended)?
        };
        let source = if prefix.is_empty() {
            translated
        } else {
            format!("(?{prefix}){translated}")
        };
        let regex = regex::Regex::new(&source)
            .map_err(|error| RegexError::Unsupported(error.to_string()))?;
        let matches_empty = regex.is_match("");
        Ok(Self {
            regex,
            matches_empty,
        })
    }
}

/// XML Schema `\s`: only the four XML whitespace characters.
const SPACE: &str = r"\x20\t\n\r";
/// XML Schema `\i`: an XML NameStartChar (including `:`).
const NAME_START: &str = r"A-Z_a-z:\xC0-\xD6\xD8-\xF6\xF8-\x{2FF}\x{370}-\x{37D}\x{37F}-\x{1FFF}\x{200C}-\x{200D}\x{2070}-\x{218F}\x{2C00}-\x{2FEF}\x{3001}-\x{D7FF}\x{F900}-\x{FDCF}\x{FDF0}-\x{FFFD}\x{10000}-\x{EFFFF}";
/// XML Schema `\c` adds these to `\i`.
const NAME_EXTRA: &str = r"\-.0-9\xB7\x{300}-\x{36F}\x{203F}-\x{2040}";

fn translate(pattern: &str, dot_all: bool, extended: bool) -> Result<String, RegexError> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut class_depth = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if extended && class_depth == 0 && matches!(c, ' ' | '\t' | '\n' | '\r') {
            i += 1;
            continue;
        }
        match c {
            '\\' => {
                let Some(&escaped) = chars.get(i + 1) else {
                    return invalid("trailing backslash");
                };
                i += 2;
                match escaped {
                    'n' | 'r' | 't' | '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{'
                    | '}' | '[' | ']' | '^' | '$' | '-' => {
                        out.push('\\');
                        out.push(escaped);
                    }
                    'd' | 'D' => {
                        out.push('\\');
                        out.push(escaped);
                    }
                    's' => out.push_str(&class(SPACE, false)),
                    'S' => out.push_str(&class(SPACE, true)),
                    'i' => out.push_str(&class(NAME_START, false)),
                    'I' => out.push_str(&class(NAME_START, true)),
                    'c' => out.push_str(&class(&format!("{NAME_START}{NAME_EXTRA}"), false)),
                    'C' => out.push_str(&class(&format!("{NAME_START}{NAME_EXTRA}"), true)),
                    'w' => out.push_str(&class(r"\p{P}\p{Z}\p{C}", true)),
                    'W' => out.push_str(&class(r"\p{P}\p{Z}\p{C}", false)),
                    'p' | 'P' => {
                        if chars.get(i) != Some(&'{') {
                            return invalid(format!("`\\{escaped}` needs a `{{…}}` property"));
                        }
                        let Some(end) = chars[i..].iter().position(|&c| c == '}') else {
                            return invalid("unterminated `\\p{`");
                        };
                        let name: String = chars[i + 1..i + end].iter().collect();
                        i += end + 1;
                        if name.starts_with("Is") {
                            return Err(RegexError::Unsupported(format!(
                                "Unicode block `{name}` is not supported"
                            )));
                        }
                        out.push('\\');
                        out.push(escaped);
                        out.push('{');
                        out.push_str(&name);
                        out.push('}');
                    }
                    '1'..='9' => {
                        return Err(RegexError::Unsupported(
                            "back-references are not supported".into(),
                        ));
                    }
                    other => return invalid(format!("invalid escape `\\{other}`")),
                }
                continue;
            }
            '[' => {
                class_depth += 1;
                out.push('[');
                if chars.get(i + 1) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                i += 1;
                continue;
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                out.push(']');
            }
            // Character class subtraction: `[a-z-[aeiou]]`.
            '-' if class_depth > 0 && chars.get(i + 1) == Some(&'[') => {
                out.push_str("--");
            }
            '.' if class_depth == 0 => {
                out.push_str(if dot_all { r"(?s:.)" } else { r"[^\n\r]" });
            }
            '(' if class_depth == 0 && chars.get(i + 1) == Some(&'?') => {
                if chars.get(i + 2) == Some(&':') {
                    out.push_str("(?:");
                    i += 3;
                    continue;
                }
                return invalid("`(?` groups other than `(?:` are not allowed");
            }
            // Characters the `regex` crate treats specially inside classes
            // (or always) but XML Schema doesn't.
            '&' | '~' if class_depth > 0 => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
        i += 1;
    }
    if class_depth > 0 {
        return invalid("unterminated character class");
    }
    Ok(out)
}

/// A class of `members`, possibly negated (the `regex` crate allows it
/// nested inside another class too).
fn class(members: &str, negated: bool) -> String {
    format!("[{}{members}]", if negated { "^" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, flags: &str, text: &str) -> bool {
        CompiledRegex::new(pattern, flags)
            .unwrap()
            .regex
            .is_match(text)
    }

    #[test]
    fn translates_xml_schema_syntax() {
        assert!(matches(
            "^[.a-z0-9]+;[a-zA-Z0-9]*;[a-zA-Z0-9_]+$",
            "",
            "p.oxy;1;23"
        ));
        assert!(!matches(
            "^[.a-z0-9]+;[a-zA-Z0-9]*;[a-zA-Z0-9_]+$",
            "",
            "p.oxy;1"
        ));
        assert!(matches(
            r"^(?:[a-zA-Z0-9]|[-._~]|[:/?#\[\]@]|[!\$()*+,;=])+$",
            "",
            "https://x.org/a?b=c"
        ));
        assert!(!matches(
            r"^(?:[a-zA-Z0-9]|[-._~]|[:/?#\[\]@]|[!\$()*+,;=])+$",
            "",
            "a b"
        ));
        assert!(matches(r"\s", "", "a b"));
        assert!(!matches(r"\s", "", "a\u{a0}b"));
        assert!(matches(r"^\i\c*$", "", "tei:div"));
        assert!(matches("[a-z-[aeiou]]", "", "b"));
        assert!(!matches("^[a-z-[aeiou]]$", "", "a"));
        assert!(!matches("a.c", "", "a\nc"));
        assert!(matches("a.c", "s", "a\nc"));
        assert!(matches("A B", "ix", "ab"));
        assert!(matches(r"\w", "", "é"));
        assert!(!matches(r"\w", "", "-"));
    }

    #[test]
    fn rejects_what_it_cannot_translate() {
        let unsupported = |pattern| {
            matches!(
                CompiledRegex::new(pattern, ""),
                Err(RegexError::Unsupported(_))
            )
        };
        assert!(unsupported(r"(a)\1"));
        assert!(unsupported(r"\p{IsBasicLatin}"));
    }

    #[test]
    fn tells_invalid_expressions_apart() {
        let code = |pattern, flags| match CompiledRegex::new(pattern, flags) {
            Err(RegexError::Invalid(code, _)) => code,
            _ => "",
        };
        assert_eq!(code(r"\,", ""), "FORX0002");
        assert_eq!(code(r"a\", ""), "FORX0002");
        assert_eq!(code("[a", ""), "FORX0002");
        assert_eq!(code("(?i)a", ""), "FORX0002");
        assert_eq!(code("a", "z"), "FORX0001");
    }
}
