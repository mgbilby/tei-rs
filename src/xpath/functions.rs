//! The XPath 2.0 (and XSLT `current()`) functions the evaluator supports.

use std::sync::Arc;

use super::XError;
use super::ast::Expr;
use super::eval::{
    Evaluator, Focus, atomize, boolean, context_item, context_node, effective_boolean,
    numeric_operand, sorted_nodes,
};
use super::parse::CompileError;
use super::regex::CompiledRegex;
use super::value::{Atomic, AtomicType, Item, Seq, XML_NS, XNode, collapse_whitespace};

type Result<T> = std::result::Result<T, XError>;

#[derive(Clone, Debug)]
pub(crate) enum Function {
    Abs,
    Avg,
    Boolean,
    Ceiling,
    CodepointsToString,
    Compare,
    Concat,
    Contains,
    Count,
    Current,
    Data,
    DistinctValues,
    DocAvailable,
    DocumentUri,
    Empty,
    EndsWith,
    Error,
    ExactlyOne,
    Exists,
    False,
    Floor,
    GenerateId,
    Head,
    Id,
    IndexOf,
    InsertBefore,
    Lang,
    Last,
    LocalName,
    LowerCase,
    Matches,
    Max,
    Min,
    Name,
    NamespaceUri,
    NormalizeSpace,
    Not,
    Number,
    OneOrMore,
    Position,
    Remove,
    Replace,
    Reverse,
    Root,
    Round,
    StartsWith,
    String,
    StringJoin,
    StringLength,
    StringToCodepoints,
    Subsequence,
    Substring,
    SubstringAfter,
    SubstringBefore,
    Sum,
    Tail,
    Tokenize,
    Trace,
    Translate,
    True,
    Unordered,
    UpperCase,
    ZeroOrOne,
    /// A regular expression argument compiled when the expression was.
    CompiledRegex(Arc<CompiledRegex>),
}

impl Function {
    /// The function with this local name in the standard function
    /// namespace, checking the number of arguments.
    pub(crate) fn from_name(name: &str, arity: usize) -> std::result::Result<Self, CompileError> {
        use Function::*;
        let (function, min, max) = match name {
            "abs" => (Abs, 1, 1),
            "avg" => (Avg, 1, 1),
            "boolean" => (Boolean, 1, 1),
            "ceiling" => (Ceiling, 1, 1),
            "codepoints-to-string" => (CodepointsToString, 1, 1),
            "compare" => (Compare, 2, 2),
            "concat" => (Concat, 2, usize::MAX),
            "contains" => (Contains, 2, 2),
            "count" => (Count, 1, 1),
            "current" => (Current, 0, 0),
            "data" => (Data, 0, 1),
            "distinct-values" => (DistinctValues, 1, 1),
            "doc-available" => (DocAvailable, 1, 1),
            "document-uri" => (DocumentUri, 0, 1),
            "empty" => (Empty, 1, 1),
            "ends-with" => (EndsWith, 2, 2),
            "error" => (Error, 0, 3),
            "exactly-one" => (ExactlyOne, 1, 1),
            "exists" => (Exists, 1, 1),
            "false" => (False, 0, 0),
            "floor" => (Floor, 1, 1),
            "generate-id" => (GenerateId, 0, 1),
            "head" => (Head, 1, 1),
            "id" => (Id, 1, 2),
            "index-of" => (IndexOf, 2, 2),
            "insert-before" => (InsertBefore, 3, 3),
            "lang" => (Lang, 1, 2),
            "last" => (Last, 0, 0),
            "local-name" => (LocalName, 0, 1),
            "lower-case" => (LowerCase, 1, 1),
            "matches" => (Matches, 2, 3),
            "max" => (Max, 1, 1),
            "min" => (Min, 1, 1),
            "name" => (Name, 0, 1),
            "namespace-uri" => (NamespaceUri, 0, 1),
            "normalize-space" => (NormalizeSpace, 0, 1),
            "not" => (Not, 1, 1),
            "number" => (Number, 0, 1),
            "one-or-more" => (OneOrMore, 1, 1),
            "position" => (Position, 0, 0),
            "remove" => (Remove, 2, 2),
            "replace" => (Replace, 3, 4),
            "reverse" => (Reverse, 1, 1),
            "root" => (Root, 0, 1),
            "round" => (Round, 1, 1),
            "starts-with" => (StartsWith, 2, 2),
            "string" => (String, 0, 1),
            "string-join" => (StringJoin, 1, 2),
            "string-length" => (StringLength, 0, 1),
            "string-to-codepoints" => (StringToCodepoints, 1, 1),
            "subsequence" => (Subsequence, 2, 3),
            "substring" => (Substring, 2, 3),
            "substring-after" => (SubstringAfter, 2, 2),
            "substring-before" => (SubstringBefore, 2, 2),
            "sum" => (Sum, 1, 2),
            "tail" => (Tail, 1, 1),
            "tokenize" => (Tokenize, 1, 3),
            "trace" => (Trace, 1, 2),
            "translate" => (Translate, 3, 3),
            "true" => (True, 0, 0),
            "unordered" => (Unordered, 1, 1),
            "upper-case" => (UpperCase, 1, 1),
            "zero-or-one" => (ZeroOrOne, 1, 1),
            _ => return Err(CompileError(format!("function {name}() is not supported"))),
        };
        if arity < min || arity > max {
            return Err(CompileError(format!(
                "{name}() with {arity} argument{} is not supported",
                if arity == 1 { "" } else { "s" }
            )));
        }
        Ok(function)
    }

    pub(crate) fn uses_position(&self) -> bool {
        matches!(self, Self::Last | Self::Position)
    }

    pub(crate) fn may_return_number(&self) -> bool {
        matches!(
            self,
            Self::Abs
                | Self::Avg
                | Self::Ceiling
                | Self::Compare
                | Self::Count
                | Self::Data
                | Self::Floor
                | Self::IndexOf
                | Self::Max
                | Self::Min
                | Self::Number
                | Self::Round
                | Self::StringLength
                | Self::StringToCodepoints
                | Self::Sum
                | Self::Head
                | Self::ExactlyOne
                | Self::ZeroOrOne
                | Self::OneOrMore
                | Self::Subsequence
        )
    }

    /// For regex functions: the indexes of the pattern and flags arguments.
    pub(crate) fn regex_arguments(&self) -> Option<(usize, usize)> {
        match self {
            Self::Matches | Self::Tokenize => Some((1, 2)),
            Self::Replace => Some((1, 3)),
            _ => None,
        }
    }
}

pub(crate) fn call<'a>(
    evaluator: &mut Evaluator<'_, 'a>,
    function: &Function,
    args: &[Expr],
    focus: Option<&Focus<'a>>,
) -> Result<Seq<'a>> {
    use Function as F;
    macro_rules! arg {
        ($index:expr) => {
            evaluator.eval(&args[$index], focus)
        };
    }
    let single = |value: Item<'a>| Ok(vec![value]);
    let string = |text: std::string::String| -> Result<Seq<'a>> {
        Ok(vec![Item::Atomic(Atomic::Str(text))])
    };
    let number = |value: f64| -> Result<Seq<'a>> { Ok(vec![Item::Atomic(Atomic::Dbl(value))]) };
    let integer = |value: i64| -> Result<Seq<'a>> { Ok(vec![Item::Atomic(Atomic::Int(value))]) };
    match function {
        F::True => Ok(boolean(true)),
        F::False => Ok(boolean(false)),
        F::Not => Ok(boolean(!effective_boolean(&arg!(0)?)?)),
        F::Boolean => Ok(boolean(effective_boolean(&arg!(0)?)?)),
        F::Count => integer(arg!(0)?.len() as i64),
        F::Exists => Ok(boolean(!arg!(0)?.is_empty())),
        F::Empty => Ok(boolean(arg!(0)?.is_empty())),
        F::Last => Ok(vec![Item::Atomic(Atomic::Int(
            focus.ok_or_else(no_focus)?.size as i64,
        ))]),
        F::Position => Ok(vec![Item::Atomic(Atomic::Int(
            focus.ok_or_else(no_focus)?.position as i64,
        ))]),
        F::Current => match evaluator.current {
            Some(node) => single(Item::Node(node)),
            None => Err(XError::new("XTDE1360", "current() is not available here")),
        },
        F::String => {
            let value = if args.is_empty() {
                vec![context_item(focus)?.clone()]
            } else {
                arg!(0)?
            };
            string(optional_string(&value)?)
        }
        F::Data => {
            let value = if args.is_empty() {
                vec![context_item(focus)?.clone()]
            } else {
                arg!(0)?
            };
            Ok(atomize(value).into_iter().map(Item::Atomic).collect())
        }
        F::StringLength => {
            let text = if args.is_empty() {
                context_item(focus)?.string()
            } else {
                optional_string(&arg!(0)?)?
            };
            integer(text.chars().count() as i64)
        }
        F::NormalizeSpace => {
            let text = if args.is_empty() {
                context_item(focus)?.string()
            } else {
                optional_string(&arg!(0)?)?
            };
            string(collapse_whitespace(&text))
        }
        F::Concat => {
            let mut text = std::string::String::new();
            for expr in args {
                text.push_str(&optional_string(&evaluator.eval(expr, focus)?)?);
            }
            string(text)
        }
        F::StringJoin => {
            let items = atomize(arg!(0)?);
            let separator = if args.len() > 1 {
                optional_string(&arg!(1)?)?
            } else {
                std::string::String::new()
            };
            let parts: Vec<std::string::String> =
                items.iter().map(Atomic::to_string_value).collect();
            string(parts.join(&separator))
        }
        F::Contains | F::StartsWith | F::EndsWith | F::SubstringBefore | F::SubstringAfter => {
            let text = optional_string(&arg!(0)?)?;
            let part = optional_string(&arg!(1)?)?;
            match function {
                F::Contains => Ok(boolean(text.contains(&part))),
                F::StartsWith => Ok(boolean(text.starts_with(&part))),
                F::EndsWith => Ok(boolean(text.ends_with(&part))),
                F::SubstringBefore => string(
                    text.find(&part)
                        .map_or_else(Default::default, |at| text[..at].to_owned()),
                ),
                _ => string(
                    text.find(&part)
                        .map_or_else(Default::default, |at| text[at + part.len()..].to_owned()),
                ),
            }
        }
        F::Substring => {
            let text = optional_string(&arg!(0)?)?;
            let start = round_half_up(required_number(&arg!(1)?)?);
            let end = if args.len() > 2 {
                start + round_half_up(required_number(&arg!(2)?)?)
            } else {
                f64::INFINITY
            };
            string(
                text.chars()
                    .enumerate()
                    .filter(|(index, _)| {
                        let position = (*index + 1) as f64;
                        position >= start && position < end
                    })
                    .map(|(_, c)| c)
                    .collect(),
            )
        }
        F::Translate => {
            let text = optional_string(&arg!(0)?)?;
            let from: Vec<char> = optional_string(&arg!(1)?)?.chars().collect();
            let to: Vec<char> = optional_string(&arg!(2)?)?.chars().collect();
            string(
                text.chars()
                    .filter_map(|c| match from.iter().position(|&f| f == c) {
                        Some(index) => to.get(index).copied(),
                        None => Some(c),
                    })
                    .collect(),
            )
        }
        F::UpperCase => string(optional_string(&arg!(0)?)?.to_uppercase()),
        F::LowerCase => string(optional_string(&arg!(0)?)?.to_lowercase()),
        F::Compare => {
            let (a, b) = (arg!(0)?, arg!(1)?);
            if a.is_empty() || b.is_empty() {
                return Ok(Vec::new());
            }
            let ordering = optional_string(&a)?.cmp(&optional_string(&b)?);
            integer(ordering as i64)
        }
        F::StringToCodepoints => Ok(optional_string(&arg!(0)?)?
            .chars()
            .map(|c| Item::Atomic(Atomic::Int(c as i64)))
            .collect()),
        F::CodepointsToString => {
            let mut text = std::string::String::new();
            for value in atomize(arg!(0)?) {
                let code = match numeric_operand(value)? {
                    Atomic::Int(code) => code,
                    other => other.as_f64().unwrap_or(-1.0) as i64,
                };
                text.push(
                    u32::try_from(code)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| {
                            XError::new("FOCH0001", format!("{code} is not a character"))
                        })?,
                );
            }
            string(text)
        }
        F::Matches => {
            let text = optional_string(&arg!(0)?)?;
            let regex = regex_argument(evaluator, args, 1, 2, focus)?;
            Ok(boolean(regex.regex.is_match(&text)))
        }
        F::Replace => {
            let text = optional_string(&arg!(0)?)?;
            let replacement = optional_string(&arg!(2)?)?;
            let regex = regex_argument(evaluator, args, 1, 3, focus)?;
            if regex.matches_empty {
                return Err(XError::new(
                    "FORX0003",
                    "the regular expression matches an empty string",
                ));
            }
            let replacement = translate_replacement(&replacement, regex.regex.captures_len())?;
            string(
                regex
                    .regex
                    .replace_all(&text, replacement.as_str())
                    .into_owned(),
            )
        }
        F::Tokenize => {
            let text = optional_string(&arg!(0)?)?;
            if args.len() == 1 {
                return Ok(text
                    .split([' ', '\t', '\n', '\r'])
                    .filter(|part| !part.is_empty())
                    .map(|part| Item::Atomic(Atomic::Str(part.to_owned())))
                    .collect());
            }
            if text.is_empty() {
                return Ok(Vec::new());
            }
            let regex = regex_argument(evaluator, args, 1, 2, focus)?;
            if regex.matches_empty {
                return Err(XError::new(
                    "FORX0003",
                    "the regular expression matches an empty string",
                ));
            }
            Ok(regex
                .regex
                .split(&text)
                .map(|part| Item::Atomic(Atomic::Str(part.to_owned())))
                .collect())
        }
        F::Number => {
            let value = if args.is_empty() {
                vec![context_item(focus)?.clone()]
            } else {
                arg!(0)?
            };
            number(match atomize(value).first() {
                Some(value) => value.to_number(),
                None => f64::NAN,
            })
        }
        F::Abs | F::Floor | F::Ceiling | F::Round => {
            let Some(value) = atomize(arg!(0)?).pop() else {
                return Ok(Vec::new());
            };
            let value = numeric_operand(value)?;
            let apply = |x: f64| match function {
                F::Abs => x.abs(),
                F::Floor => x.floor(),
                F::Ceiling => x.ceil(),
                _ => round_half_up(x),
            };
            Ok(vec![Item::Atomic(match value {
                Atomic::Int(x) => Atomic::Int(if matches!(function, F::Abs) {
                    x.abs()
                } else {
                    x
                }),
                Atomic::Dec(x) => Atomic::Dec(apply(x)),
                Atomic::Dbl(x) => Atomic::Dbl(apply(x)),
                _ => unreachable!("numeric"),
            })])
        }
        F::Sum | F::Avg => {
            let values = atomize(arg!(0)?);
            if values.is_empty() {
                return if matches!(function, F::Sum) {
                    if args.len() > 1 { arg!(1) } else { integer(0) }
                } else {
                    Ok(Vec::new())
                };
            }
            let count = values.len();
            let mut all_integers = true;
            let mut total_int: i64 = 0;
            let mut total = 0.0;
            let mut double = false;
            for value in values {
                let value = numeric_operand(value)?;
                match value {
                    Atomic::Int(x) => {
                        total_int = total_int.wrapping_add(x);
                        total += x as f64;
                    }
                    Atomic::Dbl(x) => {
                        all_integers = false;
                        double = true;
                        total += x;
                    }
                    other => {
                        all_integers = false;
                        total += other.as_f64().unwrap_or_default();
                    }
                }
            }
            let result = if matches!(function, F::Avg) {
                let mean = total / count as f64;
                if double {
                    Atomic::Dbl(mean)
                } else {
                    Atomic::Dec(mean)
                }
            } else if all_integers {
                Atomic::Int(total_int)
            } else if double {
                Atomic::Dbl(total)
            } else {
                Atomic::Dec(total)
            };
            Ok(vec![Item::Atomic(result)])
        }
        F::Min | F::Max => {
            let values = atomize(arg!(0)?);
            let mut best: Option<Atomic> = None;
            for value in values {
                let value = match value {
                    Atomic::Untyped(_) => value.cast(AtomicType::Double)?,
                    other => other,
                };
                best = Some(match best {
                    None => value,
                    Some(current) => {
                        let ordering = value.compare(&current).ok_or_else(|| {
                            XError::new("FORG0006", "min()/max() of values that can't be compared")
                        })?;
                        let better = if matches!(function, F::Min) {
                            ordering.is_lt()
                        } else {
                            ordering.is_gt()
                        };
                        if better { value } else { current }
                    }
                });
            }
            Ok(best.map(Item::Atomic).into_iter().collect())
        }
        F::DistinctValues => {
            let mut distinct: Vec<Atomic> = Vec::new();
            for value in atomize(arg!(0)?) {
                let value = match value {
                    Atomic::Untyped(text) => Atomic::Str(text),
                    other => other,
                };
                if !distinct.iter().any(|seen| atomic_equal(seen, &value)) {
                    distinct.push(value);
                }
            }
            Ok(distinct.into_iter().map(Item::Atomic).collect())
        }
        F::IndexOf => {
            let values = atomize(arg!(0)?);
            let Some(search) = atomize(arg!(1)?).pop() else {
                return Err(XError::new(
                    "XPTY0004",
                    "index-of() needs a value to search for",
                ));
            };
            let search = match search {
                Atomic::Untyped(text) => Atomic::Str(text),
                other => other,
            };
            Ok(values
                .iter()
                .enumerate()
                .filter(|(_, value)| {
                    let value = match value {
                        Atomic::Untyped(text) => Atomic::Str(text.clone()),
                        other => (*other).clone(),
                    };
                    atomic_equal(&value, &search)
                })
                .map(|(index, _)| Item::Atomic(Atomic::Int(index as i64 + 1)))
                .collect())
        }
        F::Reverse => {
            let mut items = arg!(0)?;
            items.reverse();
            Ok(items)
        }
        F::Unordered | F::Trace => arg!(0),
        F::Head => Ok(arg!(0)?.into_iter().take(1).collect()),
        F::Tail => Ok(arg!(0)?.into_iter().skip(1).collect()),
        F::Subsequence => {
            let items = arg!(0)?;
            let start = round_half_up(required_number(&arg!(1)?)?);
            let end = if args.len() > 2 {
                start + round_half_up(required_number(&arg!(2)?)?)
            } else {
                f64::INFINITY
            };
            Ok(items
                .into_iter()
                .enumerate()
                .filter(|(index, _)| {
                    let position = (*index + 1) as f64;
                    position >= start && position < end
                })
                .map(|(_, item)| item)
                .collect())
        }
        F::InsertBefore => {
            let mut items = arg!(0)?;
            let position = required_number(&arg!(1)?)? as i64;
            let inserts = arg!(2)?;
            let at = position.clamp(1, items.len() as i64 + 1) as usize - 1;
            items.splice(at..at, inserts);
            Ok(items)
        }
        F::Remove => {
            let mut items = arg!(0)?;
            let position = required_number(&arg!(1)?)? as i64;
            if position >= 1 && position as usize <= items.len() {
                items.remove(position as usize - 1);
            }
            Ok(items)
        }
        F::ZeroOrOne | F::OneOrMore | F::ExactlyOne => {
            let items = arg!(0)?;
            let ok = match function {
                F::ZeroOrOne => items.len() <= 1,
                F::OneOrMore => !items.is_empty(),
                _ => items.len() == 1,
            };
            if ok {
                Ok(items)
            } else {
                Err(XError::new("FORG0003", "wrong number of items"))
            }
        }
        F::Error => {
            let message = if args.len() > 1 {
                optional_string(&arg!(1)?)?
            } else {
                "error() called".into()
            };
            Err(XError::new("FOER0000", message))
        }
        F::Name | F::LocalName | F::NamespaceUri | F::GenerateId | F::DocumentUri | F::Root => {
            let node = if args.is_empty() {
                Some(context_node(focus)?)
            } else {
                match arg!(0)?.as_slice() {
                    [] => None,
                    [Item::Node(node)] => Some(*node),
                    _ => return Err(XError::new("XPTY0004", "expected a single node")),
                }
            };
            let Some(node) = node else {
                return match function {
                    F::Root | F::DocumentUri => Ok(Vec::new()),
                    F::NamespaceUri => Ok(vec![Item::Atomic(Atomic::Str(Default::default()))]),
                    _ => string(Default::default()),
                };
            };
            match function {
                F::Name => string(node.qname()),
                F::LocalName => string(
                    node.expanded_name()
                        .map(|(_, local)| local.to_owned())
                        .unwrap_or_default(),
                ),
                F::NamespaceUri => string(
                    node.expanded_name()
                        .and_then(|(namespace, _)| namespace)
                        .unwrap_or_default()
                        .to_owned(),
                ),
                F::GenerateId => string(match node {
                    XNode::Node(node) => format!("n{}", node.id().get()),
                    XNode::Attr(owner, index) => format!("n{}a{index}", owner.id().get()),
                }),
                F::Root => single(Item::Node(node.root())),
                _ => Ok(match (node, &evaluator.doc.uri) {
                    (XNode::Node(node), Some(uri)) if node.is_root() => {
                        vec![Item::Atomic(Atomic::Str(uri.clone()))]
                    }
                    _ => Vec::new(),
                }),
            }
        }
        F::Id => {
            let ids = atomize(arg!(0)?);
            if args.len() > 1 {
                // The node only selects the document, and there is one.
                arg!(1)?;
            } else {
                context_node(focus)?;
            }
            let mut found = Vec::new();
            for value in ids {
                for id in value.to_string_value().split_whitespace() {
                    if let Some(element) = evaluator.doc.element_by_id(id) {
                        found.push(XNode::Node(element));
                    }
                }
            }
            Ok(sorted_nodes(found))
        }
        F::Lang => {
            let wanted = optional_string(&arg!(0)?)?.to_lowercase();
            let node = if args.len() > 1 {
                match arg!(1)?.as_slice() {
                    [Item::Node(node)] => *node,
                    _ => return Err(XError::new("XPTY0004", "lang() needs a node")),
                }
            } else {
                context_node(focus)?
            };
            let language = node
                .element()
                .and_then(|element| {
                    element
                        .ancestors()
                        .find_map(|ancestor| ancestor.attribute((XML_NS, "lang")))
                })
                .map(str::to_lowercase);
            Ok(boolean(language.is_some_and(|language| {
                language == wanted
                    || language
                        .strip_prefix(&wanted)
                        .is_some_and(|rest| rest.starts_with('-'))
            })))
        }
        F::DocAvailable => {
            let uri = optional_string(&arg!(0)?)?;
            Ok(boolean(doc_available(&uri)?))
        }
        F::CompiledRegex(_) => Err(XError::new(
            "XPST0017",
            "a compiled regex is not a function",
        )),
    }
}

/// `doc-available()` for local files: whether `uri` (an absolute path or a
/// `file:` URI) names a readable, well-formed XML document. Validation never
/// touches the network, so other URIs are an error rather than a guess, as
/// are relative ones (XSLT would resolve them against the stylesheet).
/// The largest file `doc-available()` reads.
const MAX_DOCUMENT_BYTES: u64 = 256 * 1024 * 1024;

fn doc_available(uri: &str) -> Result<bool> {
    let path = match uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("file:"))
    {
        Some(path) => path,
        None if uri.contains("://") => {
            return Err(XError::new(
                "FODC0002",
                format!("doc-available(\"{uri}\"): only local files can be checked"),
            ));
        }
        None => uri,
    };
    if !std::path::Path::new(path).is_absolute() {
        return Err(XError::new(
            "FODC0002",
            format!("doc-available(\"{uri}\"): relative URIs are not supported"),
        ));
    }
    // Only regular files of a reasonable size: not devices, pipes or
    // anything that would take unbounded time or memory to read.
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES {
        return Ok(false);
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(false);
    };
    if crate::limits::xml_nesting_exceeds(&text, crate::limits::MAX_DOCUMENT_DEPTH) {
        return Ok(false);
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    Ok(roxmltree::Document::parse_with_options(&text, options).is_ok())
}

fn no_focus() -> XError {
    XError::new("XPDY0002", "there is no context item")
}

/// The string value of a sequence of at most one item ("" when empty).
fn optional_string(value: &[Item<'_>]) -> Result<String> {
    match value {
        [] => Ok(String::new()),
        [item] => Ok(item.string()),
        _ => Err(XError::new(
            "XPTY0004",
            "a sequence of more than one item where a string is required",
        )),
    }
}

fn required_number(value: &[Item<'_>]) -> Result<f64> {
    match value {
        [item] => Ok(numeric_operand(item.atomize())?
            .as_f64()
            .unwrap_or(f64::NAN)),
        _ => Err(XError::new("XPTY0004", "expected a single number")),
    }
}

/// XPath `round()`: halves round towards positive infinity.
fn round_half_up(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    (value + 0.5).floor()
}

fn atomic_equal(a: &Atomic, b: &Atomic) -> bool {
    if a.comparable(b) {
        a.compare(b).is_some_and(|ordering| ordering.is_eq())
            || (a.as_f64().is_some_and(f64::is_nan) && b.as_f64().is_some_and(f64::is_nan))
    } else {
        false
    }
}

fn regex_argument<'a>(
    evaluator: &mut Evaluator<'_, 'a>,
    args: &[Expr],
    pattern: usize,
    flags: usize,
    focus: Option<&Focus<'a>>,
) -> Result<Arc<CompiledRegex>> {
    if let Expr::Call(Function::CompiledRegex(regex), _) = &args[pattern] {
        return Ok(regex.clone());
    }
    let pattern = optional_string(&evaluator.eval(&args[pattern], focus)?)?;
    let flags = match args.get(flags) {
        Some(expr) => optional_string(&evaluator.eval(expr, focus)?)?,
        None => String::new(),
    };
    CompiledRegex::new(&pattern, &flags)
        .map(Arc::new)
        .map_err(|message| {
            XError::new(
                "FORX0002",
                format!("regular expression `{pattern}`: {message}"),
            )
        })
}

/// An XPath replacement string (`$1`, `\$`, `\\`) in the `regex` crate's
/// syntax (`${1}`, `$$`).
fn translate_replacement(replacement: &str, groups: usize) -> Result<String> {
    let mut out = String::new();
    let mut chars = replacement.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\\') => out.push('\\'),
                Some('$') => out.push_str("$$"),
                _ => {
                    return Err(XError::new(
                        "FORX0004",
                        "invalid `\\` in replacement string",
                    ));
                }
            },
            '$' => {
                let mut digits = String::new();
                while let Some(&digit) = chars.peek().filter(|c| c.is_ascii_digit()) {
                    let candidate = format!("{digits}{digit}");
                    // Take more digits only while they name a group.
                    if !digits.is_empty() && candidate.parse::<usize>().is_ok_and(|n| n >= groups) {
                        break;
                    }
                    digits = candidate;
                    chars.next();
                }
                if digits.is_empty() {
                    return Err(XError::new("FORX0004", "invalid `$` in replacement string"));
                }
                out.push_str(&format!("${{{digits}}}"));
            }
            other => out.push(other),
        }
    }
    Ok(out)
}
