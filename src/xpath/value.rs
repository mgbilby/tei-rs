//! XPath data model over a `roxmltree` document: nodes (including attribute
//! nodes, which `roxmltree` doesn't represent as nodes), atomic values, and
//! the casts and conversions between them.

use std::cmp::Ordering;
use std::fmt::Write as _;

use super::XError;
use super::datetime::{DateKind, DateTime, Duration, DurationKind};

/// The XML namespace, bound to the `xml` prefix in every document.
pub(crate) const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// A node: the document node, an element, text, comment or processing
/// instruction, or an attribute (its owner element and index).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum XNode<'a> {
    Node(roxmltree::Node<'a, 'a>),
    Attr(roxmltree::Node<'a, 'a>, u32),
}

impl<'a> XNode<'a> {
    /// Position in document order: the node's id, with an element's
    /// attributes directly after it and before its children.
    pub(crate) fn order(self) -> (u32, u32) {
        match self {
            Self::Node(node) => (node.id().get(), 0),
            Self::Attr(owner, index) => (owner.id().get(), index + 1),
        }
    }

    pub(crate) fn attribute(self) -> Option<roxmltree::Attribute<'a, 'a>> {
        match self {
            Self::Attr(owner, index) => owner.attributes().nth(index as usize),
            Self::Node(_) => None,
        }
    }

    pub(crate) fn is_element(self) -> bool {
        matches!(self, Self::Node(node) if node.is_element())
    }

    pub(crate) fn is_attribute(self) -> bool {
        matches!(self, Self::Attr(..))
    }

    /// The parent: an attribute's owner element, or the tree parent.
    pub(crate) fn parent(self) -> Option<Self> {
        match self {
            Self::Node(node) => node.parent().map(Self::Node),
            Self::Attr(owner, _) => Some(Self::Node(owner)),
        }
    }

    /// The nearest element at or above this node (an attribute's owner).
    pub(crate) fn element(self) -> Option<roxmltree::Node<'a, 'a>> {
        match self {
            Self::Node(node) => node.ancestors().find(|node| node.is_element()),
            Self::Attr(owner, _) => Some(owner),
        }
    }

    /// The root of the tree (the document node).
    pub(crate) fn root(self) -> Self {
        let node = match self {
            Self::Node(node) | Self::Attr(node, _) => node,
        };
        Self::Node(node.document().root())
    }

    /// Namespace URI and local name, for elements and attributes.
    pub(crate) fn expanded_name(self) -> Option<(Option<&'a str>, &'a str)> {
        match self {
            Self::Node(node) if node.is_element() => {
                let name = node.tag_name();
                Some((name.namespace(), name.name()))
            }
            Self::Node(node) if node.is_pi() => node.pi().map(|pi| (None, pi.target)),
            Self::Node(_) => None,
            Self::Attr(..) => {
                let attribute = self.attribute()?;
                Some((attribute.namespace(), attribute.name()))
            }
        }
    }

    /// The lexical QName, as `name()` returns it.
    pub(crate) fn qname(self) -> String {
        let Some((namespace, local)) = self.expanded_name() else {
            return String::new();
        };
        let Some(namespace) = namespace else {
            return local.to_owned();
        };
        let prefix = if namespace == XML_NS {
            Some("xml")
        } else {
            let scope = self.element();
            match self {
                // An unprefixed attribute is in no namespace, so a
                // namespaced one always has a prefix.
                Self::Attr(..) => scope.and_then(|element| prefix_for(element, namespace, true)),
                Self::Node(_) => scope.and_then(|element| prefix_for(element, namespace, false)),
            }
        };
        match prefix {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{local}"),
            _ => local.to_owned(),
        }
    }

    /// The string value.
    pub(crate) fn string_value(self) -> String {
        match self {
            Self::Attr(..) => self
                .attribute()
                .map(|attribute| attribute.value().to_owned())
                .unwrap_or_default(),
            Self::Node(node) => match node.node_type() {
                roxmltree::NodeType::Root | roxmltree::NodeType::Element => {
                    let mut text = String::new();
                    for descendant in node.descendants() {
                        if descendant.is_text() {
                            text.push_str(descendant.text().unwrap_or_default());
                        }
                    }
                    text
                }
                roxmltree::NodeType::PI => node
                    .pi()
                    .and_then(|pi| pi.value)
                    .unwrap_or_default()
                    .to_owned(),
                roxmltree::NodeType::Text | roxmltree::NodeType::Comment => {
                    node.text().unwrap_or_default().to_owned()
                }
            },
        }
    }

    /// Byte offset where the node starts in the source text.
    pub(crate) fn source_offset(self) -> usize {
        match self {
            Self::Node(node) => node.range().start,
            Self::Attr(..) => self
                .attribute()
                .map(|attribute| attribute.range().start)
                .unwrap_or_default(),
        }
    }
}

/// The prefix the document binds to `namespace` in scope at `element`; a
/// default namespace (empty prefix) only counts for elements.
fn prefix_for<'a>(
    element: roxmltree::Node<'a, 'a>,
    namespace: &str,
    attribute: bool,
) -> Option<&'a str> {
    element
        .namespaces()
        .filter(|binding| binding.uri() == namespace)
        .map(|binding| binding.name().unwrap_or(""))
        .find(|prefix| !(attribute && prefix.is_empty()))
}

/// An atomic value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Atomic {
    /// `xs:untypedAtomic`: the typed value of a node.
    Untyped(String),
    Str(String),
    Int(i64),
    /// `xs:decimal`, approximated by a double.
    Dec(f64),
    Dbl(f64),
    Bool(bool),
    /// `xs:dateTime`, `xs:date`, `xs:time` and the `xs:g…` types.
    DateTime(DateTime),
    /// `xs:duration`, `xs:dayTimeDuration` and `xs:yearMonthDuration`.
    Duration(Duration),
}

/// An item in a sequence.
#[derive(Clone, PartialEq)]
pub(crate) enum Item<'a> {
    Node(XNode<'a>),
    Atomic(Atomic),
}

pub(crate) type Seq<'a> = Vec<Item<'a>>;

impl Item<'_> {
    pub(crate) fn atomize(&self) -> Atomic {
        match self {
            Self::Node(node) => Atomic::Untyped(node.string_value()),
            Self::Atomic(atomic) => atomic.clone(),
        }
    }

    pub(crate) fn string(&self) -> String {
        match self {
            Self::Node(node) => node.string_value(),
            Self::Atomic(atomic) => atomic.to_string_value(),
        }
    }
}

/// Atomic types that casts and constructor functions can target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicType {
    String,
    Untyped,
    Boolean,
    Integer,
    Decimal,
    Double,
    AnyUri,
    DateTime(DateKind),
    Duration(DurationKind),
    /// `xs:NCName`, `xs:Name`, `xs:token`, … strings with a lexical check.
    Restricted(StringRestriction),
    /// `xs:nonNegativeInteger` and friends: an integer with a range check.
    IntegerRange(i64, i64),
    /// `xs:anyAtomicType` in sequence types.
    AnyAtomic,
    /// `xs:numeric` (XPath 3.1), in sequence types only.
    Numeric,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StringRestriction {
    NormalizedString,
    Token,
    Language,
    Name,
    NcName,
    NmToken,
    Id,
}

impl AtomicType {
    /// The type for a local name in the XML Schema namespace.
    pub(crate) fn from_xs_name(local: &str) -> Option<Self> {
        use StringRestriction::*;
        Some(match local {
            "string" => Self::String,
            "untypedAtomic" => Self::Untyped,
            "boolean" => Self::Boolean,
            "integer" | "long" => Self::Integer,
            "int" => Self::IntegerRange(i32::MIN as i64, i32::MAX as i64),
            "short" => Self::IntegerRange(i16::MIN as i64, i16::MAX as i64),
            "byte" => Self::IntegerRange(i8::MIN as i64, i8::MAX as i64),
            "nonNegativeInteger" | "unsignedLong" => Self::IntegerRange(0, i64::MAX),
            "positiveInteger" => Self::IntegerRange(1, i64::MAX),
            "nonPositiveInteger" => Self::IntegerRange(i64::MIN, 0),
            "negativeInteger" => Self::IntegerRange(i64::MIN, -1),
            "unsignedInt" => Self::IntegerRange(0, u32::MAX as i64),
            "unsignedShort" => Self::IntegerRange(0, u16::MAX as i64),
            "unsignedByte" => Self::IntegerRange(0, u8::MAX as i64),
            "decimal" => Self::Decimal,
            "double" | "float" => Self::Double,
            "anyURI" => Self::AnyUri,
            "dateTime" => Self::DateTime(DateKind::DateTime),
            "date" => Self::DateTime(DateKind::Date),
            "time" => Self::DateTime(DateKind::Time),
            "gYearMonth" => Self::DateTime(DateKind::GYearMonth),
            "gYear" => Self::DateTime(DateKind::GYear),
            "gMonthDay" => Self::DateTime(DateKind::GMonthDay),
            "gDay" => Self::DateTime(DateKind::GDay),
            "gMonth" => Self::DateTime(DateKind::GMonth),
            "duration" => Self::Duration(DurationKind::Duration),
            "dayTimeDuration" => Self::Duration(DurationKind::DayTime),
            "yearMonthDuration" => Self::Duration(DurationKind::YearMonth),
            "normalizedString" => Self::Restricted(NormalizedString),
            "token" => Self::Restricted(Token),
            "language" => Self::Restricted(Language),
            "Name" => Self::Restricted(Name),
            "NCName" => Self::Restricted(NcName),
            "NMTOKEN" => Self::Restricted(NmToken),
            "ID" | "IDREF" | "ENTITY" => Self::Restricted(Id),
            "anyAtomicType" => Self::AnyAtomic,
            "numeric" => Self::Numeric,
            _ => return None,
        })
    }

    /// Whether `value` is an instance of this type.
    pub(crate) fn admits(self, value: &Atomic) -> bool {
        match (self, value) {
            (Self::AnyAtomic, _) => true,
            (Self::Numeric, Atomic::Int(_) | Atomic::Dec(_) | Atomic::Dbl(_)) => true,
            (Self::String | Self::AnyUri | Self::Restricted(_), Atomic::Str(_)) => true,
            (Self::Untyped, Atomic::Untyped(_)) => true,
            (Self::Boolean, Atomic::Bool(_)) => true,
            (Self::Integer, Atomic::Int(_)) => true,
            (Self::IntegerRange(low, high), Atomic::Int(value)) => (low..=high).contains(value),
            (Self::Decimal, Atomic::Int(_) | Atomic::Dec(_)) => true,
            (Self::Double, Atomic::Dbl(_)) => true,
            (Self::DateTime(kind), Atomic::DateTime(value)) => value.kind == kind,
            (Self::Duration(DurationKind::Duration), Atomic::Duration(_)) => true,
            (Self::Duration(kind), Atomic::Duration(value)) => value.kind == kind,
            _ => false,
        }
    }
}

fn cast_error(value: &Atomic, target: &str) -> XError {
    XError::new(
        "FORG0001",
        format!("cannot cast `{}` to {target}", value.to_string_value()),
    )
}

/// A cast XPath never allows between the two types, whatever the value.
fn impossible_cast(value: &Atomic, target: &str) -> XError {
    XError::new(
        "XPTY0004",
        format!(
            "casting `{}` to {target} can never succeed",
            value.to_string_value()
        ),
    )
}

impl Atomic {
    pub(crate) fn is_numeric(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Dec(_) | Self::Dbl(_))
    }

    pub(crate) fn is_stringlike(&self) -> bool {
        matches!(self, Self::Str(_) | Self::Untyped(_))
    }

    /// The value as a string (`fn:string`).
    pub(crate) fn to_string_value(&self) -> String {
        match self {
            Self::Untyped(text) | Self::Str(text) => text.clone(),
            Self::Int(value) => value.to_string(),
            Self::Dec(value) => format_decimal(*value),
            Self::Dbl(value) => format_double(*value),
            Self::Bool(value) => value.to_string(),
            Self::DateTime(value) => value.to_string(),
            Self::Duration(value) => value.to_string(),
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Untyped(text) | Self::Str(text) => Some(text),
            _ => None,
        }
    }

    /// The numeric value of a numeric atomic.
    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(value) => Some(*value as f64),
            Self::Dec(value) | Self::Dbl(value) => Some(*value),
            _ => None,
        }
    }

    /// `fn:number`: never fails, NaN for anything that isn't a number.
    pub(crate) fn to_number(&self) -> f64 {
        match self {
            Self::Untyped(text) | Self::Str(text) => parse_double(text).unwrap_or(f64::NAN),
            Self::Bool(value) => f64::from(u8::from(*value)),
            _ => self.as_f64().unwrap_or(f64::NAN),
        }
    }

    /// `cast as`.
    pub(crate) fn cast(&self, target: AtomicType) -> Result<Atomic, XError> {
        if matches!(self, Self::DateTime(_) | Self::Duration(_))
            && matches!(
                target,
                AtomicType::Boolean
                    | AtomicType::Integer
                    | AtomicType::IntegerRange(..)
                    | AtomicType::Decimal
                    | AtomicType::Double
                    | AtomicType::AnyUri
            )
        {
            return Err(impossible_cast(self, "that type"));
        }
        match target {
            AtomicType::AnyAtomic => Ok(self.clone()),
            AtomicType::Numeric => match self {
                Self::Untyped(_) => self.cast(AtomicType::Double),
                _ if self.is_numeric() => Ok(self.clone()),
                _ => Err(cast_error(self, "xs:numeric")),
            },
            AtomicType::String | AtomicType::AnyUri => Ok(Self::Str(self.to_string_value())),
            AtomicType::Untyped => Ok(Self::Untyped(self.to_string_value())),
            AtomicType::Restricted(restriction) => {
                let text = self.to_string_value();
                let normalized = match restriction {
                    StringRestriction::NormalizedString => text.replace(['\t', '\n', '\r'], " "),
                    _ => collapse_whitespace(&text),
                };
                if restriction_admits(restriction, &normalized) {
                    Ok(Self::Str(normalized))
                } else {
                    Err(cast_error(self, "a restricted string type"))
                }
            }
            AtomicType::Boolean => match self {
                Self::Bool(value) => Ok(Self::Bool(*value)),
                Self::Untyped(text) | Self::Str(text) => match text.trim() {
                    "true" | "1" => Ok(Self::Bool(true)),
                    "false" | "0" => Ok(Self::Bool(false)),
                    _ => Err(cast_error(self, "xs:boolean")),
                },
                _ => {
                    let value = self.as_f64().unwrap_or(f64::NAN);
                    Ok(Self::Bool(value != 0.0 && !value.is_nan()))
                }
            },
            AtomicType::Integer | AtomicType::IntegerRange(..) => {
                let value = match self {
                    Self::Int(value) => *value,
                    Self::Bool(value) => i64::from(*value),
                    Self::Dec(value) | Self::Dbl(value) => {
                        if !value.is_finite() || value.abs() >= 9.2e18 {
                            return Err(cast_error(self, "xs:integer"));
                        }
                        value.trunc() as i64
                    }
                    Self::Untyped(text) | Self::Str(text) => {
                        parse_integer(text).ok_or_else(|| cast_error(self, "xs:integer"))?
                    }
                    Self::DateTime(_) | Self::Duration(_) => unreachable!("rejected above"),
                };
                if let AtomicType::IntegerRange(low, high) = target
                    && !(low..=high).contains(&value)
                {
                    return Err(cast_error(self, "a bounded integer type"));
                }
                Ok(Self::Int(value))
            }
            AtomicType::Decimal => match self {
                Self::Int(value) => Ok(Self::Dec(*value as f64)),
                Self::Dec(value) => Ok(Self::Dec(*value)),
                Self::Dbl(value) if value.is_finite() => Ok(Self::Dec(*value)),
                Self::Bool(value) => Ok(Self::Dec(f64::from(u8::from(*value)))),
                Self::Untyped(text) | Self::Str(text) => parse_decimal(text)
                    .map(Self::Dec)
                    .ok_or_else(|| cast_error(self, "xs:decimal")),
                Self::Dbl(_) => Err(cast_error(self, "xs:decimal")),
                Self::DateTime(_) | Self::Duration(_) => unreachable!("rejected above"),
            },
            AtomicType::Double => match self {
                Self::Untyped(text) | Self::Str(text) => parse_double(text)
                    .map(Self::Dbl)
                    .ok_or_else(|| cast_error(self, "xs:double")),
                Self::Bool(value) => Ok(Self::Dbl(f64::from(u8::from(*value)))),
                _ => Ok(Self::Dbl(self.as_f64().unwrap_or(f64::NAN))),
            },
            AtomicType::DateTime(kind) => match self {
                Self::Untyped(text) | Self::Str(text) => DateTime::parse(kind, text)
                    .map(Self::DateTime)
                    .ok_or_else(|| cast_error(self, kind.name())),
                Self::DateTime(value) => value
                    .convert(kind)
                    .map(Self::DateTime)
                    .ok_or_else(|| impossible_cast(self, kind.name())),
                _ => Err(impossible_cast(self, kind.name())),
            },
            AtomicType::Duration(kind) => match self {
                Self::Untyped(text) | Self::Str(text) => Duration::parse(kind, text)
                    .map(Self::Duration)
                    .ok_or_else(|| cast_error(self, kind.name())),
                Self::Duration(value) => Ok(Self::Duration(value.convert(kind))),
                _ => Err(impossible_cast(self, kind.name())),
            },
        }
    }

    /// Compares two values of comparable types (after any promotion the
    /// caller has applied); `None` when the types aren't comparable.
    pub(crate) fn compare(&self, other: &Atomic) -> Option<Ordering> {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => Some(a.cmp(b)),
            (a, b) if a.is_numeric() && b.is_numeric() => {
                a.as_f64().unwrap().partial_cmp(&b.as_f64().unwrap())
            }
            (a, b) if a.is_stringlike() && b.is_stringlike() => {
                Some(a.as_str().unwrap().cmp(b.as_str().unwrap()))
            }
            (Self::Bool(a), Self::Bool(b)) => Some(a.cmp(b)),
            (Self::DateTime(a), Self::DateTime(b)) if a.kind == b.kind => {
                Some(a.instant().cmp(&b.instant()))
            }
            (Self::Duration(a), Self::Duration(b)) => Some(a.compare(b)),
            _ => None,
        }
    }

    /// Whether the two values have types that can be compared at all.
    pub(crate) fn comparable(&self, other: &Atomic) -> bool {
        (self.is_numeric() && other.is_numeric())
            || (self.is_stringlike() && other.is_stringlike())
            || matches!((self, other), (Self::Bool(_), Self::Bool(_)))
            || matches!((self, other), (Self::DateTime(a), Self::DateTime(b)) if a.kind == b.kind)
            || matches!((self, other), (Self::Duration(_), Self::Duration(_)))
    }

    /// Whether `lt`, `gt` and friends are defined between two comparable
    /// values (the `xs:g…` types and mixed or plain `xs:duration`s only have
    /// `eq` and `ne`).
    pub(crate) fn ordered_with(&self, other: &Atomic) -> bool {
        match (self, other) {
            (Self::DateTime(a), _) => a.kind.is_ordered(),
            (Self::Duration(a), Self::Duration(b)) => a.ordered_with(b),
            _ => true,
        }
    }
}

fn restriction_admits(restriction: StringRestriction, text: &str) -> bool {
    match restriction {
        StringRestriction::NormalizedString | StringRestriction::Token => true,
        StringRestriction::Language => {
            let mut parts = text.split('-');
            parts.next().is_some_and(|first| {
                (1..=8).contains(&first.len()) && first.bytes().all(|b| b.is_ascii_alphabetic())
            }) && parts.all(|part| {
                (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
            })
        }
        StringRestriction::Name => {
            let mut chars = text.chars();
            chars.next().is_some_and(|c| is_name_start(c) || c == ':')
                && chars.all(|c| is_name_char(c) || c == ':')
        }
        StringRestriction::NcName | StringRestriction::Id => is_ncname(text),
        StringRestriction::NmToken => !text.is_empty() && text.chars().all(is_name_char),
    }
}

pub(crate) fn is_ncname(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(is_name_start) && chars.all(is_name_char)
}

/// XML 1.0 (fifth edition) NameStartChar, without `:`.
pub(crate) fn is_name_start(c: char) -> bool {
    matches!(c,
        'A'..='Z' | '_' | 'a'..='z' | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}')
}

/// XML 1.0 (fifth edition) NameChar, without `:`.
pub(crate) fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

pub(crate) fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in text
        .split([' ', '\t', '\n', '\r'])
        .filter(|word| !word.is_empty())
    {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn trim_xml(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\n', '\r'])
}

fn parse_integer(text: &str) -> Option<i64> {
    let text = trim_xml(text);
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.strip_prefix('+').unwrap_or(text).parse().ok()
}

fn parse_decimal(text: &str) -> Option<f64> {
    let text = trim_xml(text);
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
        return None;
    }
    let normalized = format!(
        "{}{}.{}0",
        if text.starts_with('-') { "-" } else { "" },
        if whole.is_empty() { "0" } else { whole },
        fraction
    );
    normalized.parse().ok()
}

/// The lexical space of `xs:double`.
pub(crate) fn parse_double(text: &str) -> Option<f64> {
    let text = trim_xml(text);
    match text {
        "INF" | "+INF" => return Some(f64::INFINITY),
        "-INF" => return Some(f64::NEG_INFINITY),
        "NaN" => return Some(f64::NAN),
        _ => {}
    }
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(at) => (&text[..at], Some(&text[at + 1..])),
        None => (text, None),
    };
    parse_decimal(mantissa)?;
    if let Some(exponent) = exponent {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    text.parse().ok()
}

/// `xs:double` to string, as XPath 2.0 prescribes.
pub(crate) fn format_double(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "INF" } else { "-INF" }.into();
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let magnitude = value.abs();
    if (1e-6..1e6).contains(&magnitude) {
        return format!("{value}");
    }
    let mut text = String::new();
    let _ = write!(text, "{value:E}");
    if let Some(at) = text.find('E')
        && !text[..at].contains('.')
    {
        text.insert_str(at, ".0");
    }
    text
}

fn format_decimal(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    format!("{value}")
}
