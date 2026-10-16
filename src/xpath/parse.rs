//! Tokenizer and recursive-descent parser for the XPath 2.0 subset (with
//! the XPath 3.0 `!`, `||`, `let` and `=>`) and for XSLT match patterns.

use std::collections::HashMap;

use super::ast::*;
use super::functions::Function;
use super::regex::{CompiledRegex, RegexError};
use super::value::{Atomic, AtomicType, XML_NS, is_name_char, is_name_start};

const XS_NS: &str = "http://www.w3.org/2001/XMLSchema";
const FN_NS: &str = "http://www.w3.org/2005/xpath-functions";

/// Why an expression can't be compiled: a syntax error, or a feature this
/// evaluator doesn't implement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompileError(pub String);

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

type Result<T> = std::result::Result<T, CompileError>;

fn error<T>(message: impl Into<String>) -> Result<T> {
    Err(CompileError(message.into()))
}

/// What names mean while parsing: namespace prefixes and in-scope
/// variables, plus the static base URI.
#[derive(Clone, Debug, Default)]
pub(crate) struct StaticContext {
    pub namespaces: HashMap<String, String>,
    pub variables: Vec<String>,
    /// What `doc()`, `document()` and `doc-available()` resolve relative
    /// URIs against: the URI of the schema file the expression is in.
    pub base_uri: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    /// A QName or NCName, possibly with a `*` wildcard part: (prefix, local).
    Name(Option<String>, String),
    /// `prefix:*`
    PrefixWildcard(String),
    /// `*:local`
    LocalWildcard(String),
    Variable(Option<String>, String),
    String(String),
    Integer(i64),
    Decimal(f64),
    Double(f64),
    Symbol(&'static str),
}

const SYMBOLS: [&str; 27] = [
    "//", "::", "..", "!=", "<=", ">=", "<<", ">>", ":=", "||", "=>", "(", ")", "[", "]", ",", "/",
    "@", ".", "|", "=", "<", ">", "+", "-", "*", "!",
];

fn tokenize(text: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let ncname = |i: &mut usize| -> Option<String> {
        if *i < chars.len() && is_name_start(chars[*i]) {
            let start = *i;
            while *i < chars.len() && is_name_char(chars[*i]) {
                *i += 1;
            }
            Some(chars[start..*i].iter().collect())
        } else {
            None
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Comments `(: … :)`, which may nest.
        if c == '(' && chars.get(i + 1) == Some(&':') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '(' && chars.get(i + 1) == Some(&':') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == ':' && chars.get(i + 1) == Some(&')') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if depth != 0 {
                return error("unterminated comment");
            }
            continue;
        }
        if c == '"' || c == '\'' {
            let mut value = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return error("unterminated string literal"),
                    Some(&q) if q == c => {
                        if chars.get(i + 1) == Some(&c) {
                            value.push(c);
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    }
                    Some(&other) => {
                        value.push(other);
                        i += 1;
                    }
                }
            }
            tokens.push(Token::String(value));
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut decimal = false;
            if i < chars.len() && chars[i] == '.' {
                decimal = true;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let mut double = false;
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    double = true;
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let literal: String = chars[start..i].iter().collect();
            if i < chars.len() && is_name_start(chars[i]) {
                return error(format!("invalid number `{literal}{}`", chars[i]));
            }
            tokens.push(if double {
                Token::Double(
                    literal
                        .parse()
                        .map_err(|_| CompileError("bad number".into()))?,
                )
            } else if decimal {
                Token::Decimal(
                    literal
                        .parse()
                        .map_err(|_| CompileError("bad number".into()))?,
                )
            } else {
                match literal.parse() {
                    Ok(value) => Token::Integer(value),
                    Err(_) => return error(format!("integer `{literal}` is too large")),
                }
            });
            continue;
        }
        if c == '$' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            let Some(first) = ncname(&mut i) else {
                return error("`$` must be followed by a variable name");
            };
            if chars.get(i) == Some(&':') && chars.get(i + 1).is_some_and(|&c| is_name_start(c)) {
                i += 1;
                let local = ncname(&mut i).expect("checked");
                tokens.push(Token::Variable(Some(first), local));
            } else {
                tokens.push(Token::Variable(None, first));
            }
            continue;
        }
        if c == '*'
            && chars.get(i + 1) == Some(&':')
            && chars.get(i + 2).is_some_and(|&c| is_name_start(c))
        {
            i += 2;
            let local = ncname(&mut i).expect("checked");
            tokens.push(Token::LocalWildcard(local));
            continue;
        }
        if let Some(first) = ncname(&mut i) {
            if chars.get(i) == Some(&':') {
                if chars.get(i + 1).is_some_and(|&c| is_name_start(c)) {
                    i += 1;
                    let local = ncname(&mut i).expect("checked");
                    tokens.push(Token::Name(Some(first), local));
                    continue;
                }
                if chars.get(i + 1) == Some(&'*') {
                    i += 2;
                    tokens.push(Token::PrefixWildcard(first));
                    continue;
                }
            }
            tokens.push(Token::Name(None, first));
            continue;
        }
        let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
        if let Some(symbol) = SYMBOLS.iter().find(|symbol| rest.starts_with(**symbol)) {
            i += symbol.chars().count();
            tokens.push(Token::Symbol(symbol));
            continue;
        }
        if c == '?' {
            i += 1;
            tokens.push(Token::Symbol("?"));
            continue;
        }
        return error(format!("unexpected character `{c}`"));
    }
    Ok(tokens)
}

/// Parses an XPath expression.
pub(crate) fn parse_expr(text: &str, context: &StaticContext) -> Result<Expr> {
    let mut parser = Parser::new(text, context)?;
    let expr = parser.expr()?;
    parser.finish()?;
    Ok(expr)
}

/// Parses an XSLT match pattern (a Schematron rule context): an XSLT 2.0
/// pattern, or an XSLT 3.0 one with parenthesised unions as steps.
pub(crate) fn parse_pattern(text: &str, context: &StaticContext) -> Result<Pattern> {
    let mut parser = Parser::new(text, context)?;
    let alternatives = parser.union_pattern()?;
    parser.finish()?;
    Ok(Pattern { alternatives })
}

struct Parser<'c> {
    tokens: Vec<Token>,
    position: usize,
    context: &'c StaticContext,
    /// Variables bound by `for`/`let`/`some`/`every` inside the expression.
    locals: Vec<String>,
    /// How many expressions enclose the current one.
    depth: usize,
}

/// How deeply expressions may nest. Every nested expression is parsed (and
/// later evaluated) recursively, so a limit keeps a malicious schema from
/// overflowing the stack; real Schematron tests nest a few levels.
const MAX_DEPTH: usize = 256;

/// How many path patterns a pattern may expand to once its parenthesised
/// unions are distributed (`(a | b)/(c | d)` is four).
const MAX_ALTERNATIVES: usize = 1024;

impl<'c> Parser<'c> {
    fn new(text: &str, context: &'c StaticContext) -> Result<Self> {
        Ok(Self {
            tokens: tokenize(text)?,
            position: 0,
            context,
            locals: Vec::new(),
            depth: 0,
        })
    }

    fn finish(&self) -> Result<()> {
        match self.peek() {
            None => Ok(()),
            Some(token) => error(format!("unexpected {}", describe(token))),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }

    fn at_symbol(&self, symbol: &str) -> bool {
        matches!(self.peek(), Some(Token::Symbol(s)) if *s == symbol)
    }

    fn eat_symbol(&mut self, symbol: &str) -> bool {
        let found = self.at_symbol(symbol);
        if found {
            self.position += 1;
        }
        found
    }

    fn expect_symbol(&mut self, symbol: &str) -> Result<()> {
        if self.eat_symbol(symbol) {
            Ok(())
        } else {
            error(format!(
                "expected `{symbol}`, found {}",
                self.peek().map_or("end of expression".into(), describe)
            ))
        }
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(self.peek(), Some(Token::Name(None, name)) if name == keyword)
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        let found = self.at_keyword(keyword);
        if found {
            self.position += 1;
        }
        found
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.eat_keyword(keyword) {
            Ok(())
        } else {
            error(format!("expected `{keyword}`"))
        }
    }

    // --- Expressions ----------------------------------------------------

    fn expr(&mut self) -> Result<Expr> {
        let first = self.expr_single()?;
        if !self.at_symbol(",") {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat_symbol(",") {
            items.push(self.expr_single()?);
        }
        Ok(Expr::Sequence(items))
    }

    fn expr_single(&mut self) -> Result<Expr> {
        if self.depth == MAX_DEPTH {
            return error(format!(
                "expression nested more than {MAX_DEPTH} levels deep"
            ));
        }
        self.depth += 1;
        let expr = self.expr_single_unchecked();
        self.depth -= 1;
        expr
    }

    fn expr_single_unchecked(&mut self) -> Result<Expr> {
        let next_is = |parser: &Self, symbol: &str| matches!(parser.peek_at(1), Some(Token::Symbol(s)) if *s == symbol);
        let next_is_variable = matches!(self.peek_at(1), Some(Token::Variable(..)));
        if self.at_keyword("for") && next_is_variable {
            self.position += 1;
            return self.bindings("in", "return", |var, domain, body| {
                Expr::For(var, Box::new(domain), Box::new(body))
            });
        }
        if self.at_keyword("let") && next_is_variable {
            self.position += 1;
            return self.bindings(":=", "return", |var, value, body| {
                Expr::Let(var, Box::new(value), Box::new(body))
            });
        }
        if (self.at_keyword("some") || self.at_keyword("every")) && next_is_variable {
            let every = self.at_keyword("every");
            self.position += 1;
            return self.bindings("in", "satisfies", move |var, domain, body| {
                Expr::Quantified(every, var, Box::new(domain), Box::new(body))
            });
        }
        if self.at_keyword("if") && next_is(self, "(") {
            self.position += 2;
            let condition = self.expr()?;
            self.expect_symbol(")")?;
            self.expect_keyword("then")?;
            let then = self.expr_single()?;
            self.expect_keyword("else")?;
            let otherwise = self.expr_single()?;
            return Ok(Expr::If(
                Box::new(condition),
                Box::new(then),
                Box::new(otherwise),
            ));
        }
        self.or_expr()
    }

    /// `$a in X, $b in Y keyword body` (or `$a := X, …` for `let`), nested
    /// right to left.
    fn bindings(
        &mut self,
        binder: &str,
        keyword: &str,
        build: impl Fn(String, Expr, Expr) -> Expr,
    ) -> Result<Expr> {
        let mut bound = Vec::new();
        loop {
            let Some(Token::Variable(prefix, name)) = self.next() else {
                return error("expected a variable binding");
            };
            if prefix.is_some() {
                return error("prefixed variable names are not supported");
            }
            if binder == ":=" {
                self.expect_symbol(binder)?;
            } else {
                self.expect_keyword(binder)?;
            }
            let domain = self.expr_single()?;
            self.locals.push(name.clone());
            bound.push((name, domain));
            if !self.eat_symbol(",") {
                break;
            }
        }
        self.expect_keyword(keyword)?;
        let mut body = self.expr_single()?;
        for (name, domain) in bound.into_iter().rev() {
            self.locals.pop();
            body = build(name, domain, body);
        }
        Ok(body)
    }

    fn or_expr(&mut self) -> Result<Expr> {
        let mut left = self.and_expr()?;
        while self.eat_keyword("or") {
            left = Expr::Or(Box::new(left), Box::new(self.and_expr()?));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr> {
        let mut left = self.comparison_expr()?;
        while self.eat_keyword("and") {
            left = Expr::And(Box::new(left), Box::new(self.comparison_expr()?));
        }
        Ok(left)
    }

    fn comparison_expr(&mut self) -> Result<Expr> {
        let left = self.string_concat_expr()?;
        let general = match self.peek() {
            Some(Token::Symbol("=")) => Some(CompareOp::Eq),
            Some(Token::Symbol("!=")) => Some(CompareOp::Ne),
            Some(Token::Symbol("<")) => Some(CompareOp::Lt),
            Some(Token::Symbol("<=")) => Some(CompareOp::Le),
            Some(Token::Symbol(">")) => Some(CompareOp::Gt),
            Some(Token::Symbol(">=")) => Some(CompareOp::Ge),
            _ => None,
        };
        if let Some(op) = general {
            self.position += 1;
            let right = self.string_concat_expr()?;
            return Ok(Expr::GeneralCompare(op, Box::new(left), Box::new(right)));
        }
        let value = match self.peek() {
            Some(Token::Name(None, name)) => match name.as_str() {
                "eq" => Some(CompareOp::Eq),
                "ne" => Some(CompareOp::Ne),
                "lt" => Some(CompareOp::Lt),
                "le" => Some(CompareOp::Le),
                "gt" => Some(CompareOp::Gt),
                "ge" => Some(CompareOp::Ge),
                _ => None,
            },
            _ => None,
        };
        if let Some(op) = value {
            self.position += 1;
            let right = self.string_concat_expr()?;
            return Ok(Expr::ValueCompare(op, Box::new(left), Box::new(right)));
        }
        let node = match self.peek() {
            Some(Token::Name(None, name)) if name == "is" => Some(NodeOp::Is),
            Some(Token::Symbol("<<")) => Some(NodeOp::Precedes),
            Some(Token::Symbol(">>")) => Some(NodeOp::Follows),
            _ => None,
        };
        if let Some(op) = node {
            self.position += 1;
            let right = self.string_concat_expr()?;
            return Ok(Expr::NodeCompare(op, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    /// `a || b || …` (XPath 3.0): `concat(a, b, …)`.
    fn string_concat_expr(&mut self) -> Result<Expr> {
        let first = self.range_expr()?;
        if !self.at_symbol("||") {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.eat_symbol("||") {
            parts.push(self.range_expr()?);
        }
        Ok(Expr::Call(Function::Concat, parts))
    }

    /// Checks that a chain of `links` operators that nest their left operand
    /// (`!`, `=>`) stays within the nesting limit, as evaluating it recurses
    /// once per link.
    fn check_chain(&self, links: usize) -> Result<()> {
        if self.depth + links > MAX_DEPTH {
            return error(format!(
                "expression nested more than {MAX_DEPTH} levels deep"
            ));
        }
        Ok(())
    }

    fn range_expr(&mut self) -> Result<Expr> {
        let left = self.additive_expr()?;
        if self.eat_keyword("to") {
            let right = self.additive_expr()?;
            return Ok(Expr::Range(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn additive_expr(&mut self) -> Result<Expr> {
        let mut left = self.multiplicative_expr()?;
        loop {
            let op = if self.eat_symbol("+") {
                ArithmeticOp::Add
            } else if self.eat_symbol("-") {
                ArithmeticOp::Subtract
            } else {
                return Ok(left);
            };
            let right = self.multiplicative_expr()?;
            left = Expr::Arithmetic(op, Box::new(left), Box::new(right));
        }
    }

    fn multiplicative_expr(&mut self) -> Result<Expr> {
        let mut left = self.union_expr()?;
        loop {
            let op = if self.eat_symbol("*") {
                ArithmeticOp::Multiply
            } else if self.eat_keyword("div") {
                ArithmeticOp::Divide
            } else if self.eat_keyword("idiv") {
                ArithmeticOp::IntegerDivide
            } else if self.eat_keyword("mod") {
                ArithmeticOp::Modulo
            } else {
                return Ok(left);
            };
            let right = self.union_expr()?;
            left = Expr::Arithmetic(op, Box::new(left), Box::new(right));
        }
    }

    fn union_expr(&mut self) -> Result<Expr> {
        let mut left = self.intersect_expr()?;
        while self.eat_symbol("|") || self.eat_keyword("union") {
            left = Expr::Union(Box::new(left), Box::new(self.intersect_expr()?));
        }
        Ok(left)
    }

    fn intersect_expr(&mut self) -> Result<Expr> {
        let mut left = self.instanceof_expr()?;
        loop {
            if self.eat_keyword("intersect") {
                left = Expr::Intersect(Box::new(left), Box::new(self.instanceof_expr()?));
            } else if self.eat_keyword("except") {
                left = Expr::Except(Box::new(left), Box::new(self.instanceof_expr()?));
            } else {
                return Ok(left);
            }
        }
    }

    fn instanceof_expr(&mut self) -> Result<Expr> {
        let expr = self.treat_expr()?;
        if self.at_keyword("instance")
            && matches!(self.peek_at(1), Some(Token::Name(None, n)) if n == "of")
        {
            self.position += 2;
            let sequence_type = self.sequence_type()?;
            return Ok(Expr::InstanceOf(Box::new(expr), sequence_type));
        }
        Ok(expr)
    }

    fn treat_expr(&mut self) -> Result<Expr> {
        let expr = self.castable_expr()?;
        if self.at_keyword("treat")
            && matches!(self.peek_at(1), Some(Token::Name(None, n)) if n == "as")
        {
            self.position += 2;
            // `treat as` only asserts a type; the value is unchanged.
            self.sequence_type()?;
        }
        Ok(expr)
    }

    fn castable_expr(&mut self) -> Result<Expr> {
        let expr = self.cast_expr()?;
        if self.at_keyword("castable")
            && matches!(self.peek_at(1), Some(Token::Name(None, n)) if n == "as")
        {
            self.position += 2;
            let (target, optional) = self.single_type()?;
            return Ok(Expr::Castable(Box::new(expr), target, optional));
        }
        Ok(expr)
    }

    fn cast_expr(&mut self) -> Result<Expr> {
        let expr = self.arrow_expr()?;
        if self.at_keyword("cast")
            && matches!(self.peek_at(1), Some(Token::Name(None, n)) if n == "as")
        {
            self.position += 2;
            let (target, optional) = self.single_type()?;
            return Ok(Expr::Cast(Box::new(expr), target, optional));
        }
        Ok(expr)
    }

    fn single_type(&mut self) -> Result<(AtomicType, bool)> {
        let Some(Token::Name(prefix, local)) = self.next() else {
            return error("expected a type name");
        };
        let target = self.atomic_type(prefix.as_deref(), &local)?;
        Ok((target, self.eat_symbol("?")))
    }

    fn atomic_type(&self, prefix: Option<&str>, local: &str) -> Result<AtomicType> {
        let namespace = match prefix {
            Some(prefix) => self.namespace(prefix)?,
            None => return error(format!("unknown type `{local}`")),
        };
        if namespace != XS_NS {
            return error(format!(
                "unknown type `{}:{local}`",
                prefix.unwrap_or_default()
            ));
        }
        AtomicType::from_xs_name(local)
            .ok_or_else(|| CompileError(format!("type xs:{local} is not supported")))
    }

    fn sequence_type(&mut self) -> Result<SequenceType> {
        if self.at_keyword("empty-sequence") {
            self.position += 1;
            self.expect_symbol("(")?;
            self.expect_symbol(")")?;
            return Ok(SequenceType {
                item: None,
                min: 0,
                many: false,
            });
        }
        let item = match self.peek().cloned() {
            Some(Token::Name(None, name))
                if name == "item" && self.peek_at(1) == Some(&Token::Symbol("(")) =>
            {
                self.position += 1;
                self.expect_symbol("(")?;
                self.expect_symbol(")")?;
                ItemType::Item
            }
            Some(Token::Name(None, name))
                if is_kind_test(&name) && self.peek_at(1) == Some(&Token::Symbol("(")) =>
            {
                self.position += 1;
                ItemType::Node(self.kind_test(&name)?)
            }
            Some(Token::Name(prefix, local)) => {
                self.position += 1;
                ItemType::Atomic(self.atomic_type(prefix.as_deref(), &local)?)
            }
            _ => return error("expected a sequence type"),
        };
        let (min, many) = if self.eat_symbol("?") {
            (0, false)
        } else if self.eat_symbol("*") {
            (0, true)
        } else if self.eat_symbol("+") {
            (1, true)
        } else {
            (1, false)
        };
        Ok(SequenceType {
            item: Some(item),
            min,
            many,
        })
    }

    /// `a => f(b)` (XPath 3.1): `f(a, b)`.
    fn arrow_expr(&mut self) -> Result<Expr> {
        let mut expr = self.unary_expr()?;
        let mut links = 0;
        while self.eat_symbol("=>") {
            links += 1;
            self.check_chain(links)?;
            let Some(Token::Name(prefix, local)) = self.next() else {
                return error("expected a function name after `=>`");
            };
            self.expect_symbol("(")?;
            let mut args = vec![expr];
            args.extend(self.arguments()?);
            expr = self.function_call(prefix.as_deref(), &local, args)?;
        }
        Ok(expr)
    }

    fn unary_expr(&mut self) -> Result<Expr> {
        let mut negate = false;
        let mut signed = false;
        loop {
            if self.eat_symbol("-") {
                negate = !negate;
                signed = true;
            } else if self.eat_symbol("+") {
                signed = true;
            } else {
                break;
            }
        }
        let expr = self.simple_map_expr()?;
        Ok(if negate {
            Expr::Negate(Box::new(expr))
        } else if signed {
            // Unary plus still requires a numeric operand.
            Expr::Arithmetic(
                ArithmeticOp::Add,
                Box::new(Expr::Literal(Atomic::Int(0))),
                Box::new(expr),
            )
        } else {
            expr
        })
    }

    /// `a ! b ! …` (XPath 3.0).
    fn simple_map_expr(&mut self) -> Result<Expr> {
        let mut expr = self.path_expr()?;
        let mut links = 0;
        while self.eat_symbol("!") {
            links += 1;
            self.check_chain(links)?;
            expr = Expr::Map(Box::new(expr), Box::new(self.path_expr()?));
        }
        Ok(expr)
    }

    fn path_expr(&mut self) -> Result<Expr> {
        if self.eat_symbol("/") {
            if self.at_step_start() {
                let relative = self.relative_path(Expr::Root)?;
                return Ok(relative);
            }
            return Ok(Expr::Root);
        }
        if self.eat_symbol("//") {
            let start = Expr::Path(Box::new(Expr::Root), Box::new(descendant_or_self()));
            return self.relative_path_after(start);
        }
        let first = self.step_expr()?;
        if self.at_symbol("/") || self.at_symbol("//") {
            return self.continue_path(first);
        }
        Ok(first)
    }

    /// Whether the next token can start a step (for a lone `/`).
    fn at_step_start(&self) -> bool {
        match self.peek() {
            Some(Token::Name(None, name)) => {
                !matches!(
                    name.as_str(),
                    "and"
                        | "or"
                        | "div"
                        | "idiv"
                        | "mod"
                        | "eq"
                        | "ne"
                        | "lt"
                        | "le"
                        | "gt"
                        | "ge"
                        | "is"
                        | "to"
                        | "union"
                        | "intersect"
                        | "except"
                        | "instance"
                        | "treat"
                        | "castable"
                        | "cast"
                        | "then"
                        | "else"
                        | "return"
                        | "satisfies"
                        | "in"
                ) || self.peek_at(1) == Some(&Token::Symbol("("))
                    || self.peek_at(1) == Some(&Token::Symbol("::"))
            }
            Some(Token::Name(..) | Token::PrefixWildcard(_) | Token::LocalWildcard(_)) => true,
            Some(Token::Symbol(symbol)) => matches!(*symbol, "@" | "." | ".." | "*"),
            _ => false,
        }
    }

    fn relative_path(&mut self, start: Expr) -> Result<Expr> {
        let step = self.step_expr()?;
        self.continue_path(join(start, step))
    }

    fn relative_path_after(&mut self, start: Expr) -> Result<Expr> {
        let step = self.step_expr()?;
        self.continue_path(join(start, step))
    }

    fn continue_path(&mut self, mut path: Expr) -> Result<Expr> {
        loop {
            if self.eat_symbol("/") {
                let step = self.step_expr()?;
                path = join(path, step);
            } else if self.eat_symbol("//") {
                let step = self.step_expr()?;
                path = join(join(path, descendant_or_self()), step);
            } else {
                return Ok(path);
            }
        }
    }

    fn step_expr(&mut self) -> Result<Expr> {
        if let Some(step) = self.axis_step()? {
            return Ok(Expr::Step(step));
        }
        let primary = self.primary_expr()?;
        let predicates = self.predicates()?;
        Ok(if predicates.is_empty() {
            primary
        } else {
            Expr::Filter(Box::new(primary), predicates)
        })
    }

    fn predicates(&mut self) -> Result<Vec<Expr>> {
        let mut predicates = Vec::new();
        while self.eat_symbol("[") {
            predicates.push(self.expr()?);
            self.expect_symbol("]")?;
        }
        Ok(predicates)
    }

    /// An axis step, or `None` if the next tokens start a primary
    /// expression instead.
    fn axis_step(&mut self) -> Result<Option<Step>> {
        if self.eat_symbol("..") {
            return Ok(Some(Step {
                axis: Axis::Parent,
                test: NodeTest::AnyNode,
                predicates: self.predicates()?,
            }));
        }
        let axis = if self.eat_symbol("@") {
            Some(Axis::Attribute)
        } else if let (Some(Token::Name(None, name)), Some(Token::Symbol("::"))) =
            (self.peek(), self.peek_at(1))
        {
            let axis = Axis::from_name(name)
                .ok_or_else(|| CompileError(format!("axis `{name}` is not supported")))?;
            self.position += 2;
            Some(axis)
        } else {
            None
        };
        let test = match self.peek().cloned() {
            Some(Token::Symbol("*")) => {
                self.position += 1;
                NodeTest::Name(NameTest {
                    namespace: None,
                    local: None,
                })
            }
            Some(Token::PrefixWildcard(prefix)) => {
                self.position += 1;
                NodeTest::Name(NameTest {
                    namespace: Some(Some(self.namespace(&prefix)?.to_owned())),
                    local: None,
                })
            }
            Some(Token::LocalWildcard(local)) => {
                self.position += 1;
                NodeTest::Name(NameTest {
                    namespace: None,
                    local: Some(local),
                })
            }
            Some(Token::Name(None, name))
                if is_kind_test(&name) && self.peek_at(1) == Some(&Token::Symbol("(")) =>
            {
                self.position += 1;
                self.kind_test(&name)?
            }
            Some(Token::Name(prefix, local)) if self.peek_at(1) != Some(&Token::Symbol("(")) => {
                self.position += 1;
                NodeTest::Name(self.name_test(prefix.as_deref(), local)?)
            }
            _ if axis.is_some() => return error("expected a node test after the axis"),
            _ => return Ok(None),
        };
        Ok(Some(Step {
            axis: axis.unwrap_or(match test {
                NodeTest::Attribute(_) => Axis::Attribute,
                _ => Axis::Child,
            }),
            test,
            predicates: self.predicates()?,
        }))
    }

    /// A QName test; unprefixed names are in no namespace, as in XSLT
    /// without `xpath-default-namespace`.
    fn name_test(&self, prefix: Option<&str>, local: String) -> Result<NameTest> {
        Ok(NameTest {
            namespace: Some(match prefix {
                Some(prefix) => Some(self.namespace(prefix)?.to_owned()),
                None => None,
            }),
            local: Some(local),
        })
    }

    fn namespace(&self, prefix: &str) -> Result<&str> {
        if prefix == "xml" {
            return Ok(XML_NS);
        }
        self.context
            .namespaces
            .get(prefix)
            .map(String::as_str)
            .ok_or_else(|| CompileError(format!("undeclared namespace prefix `{prefix}`")))
    }

    fn kind_test(&mut self, name: &str) -> Result<NodeTest> {
        self.expect_symbol("(")?;
        let test = match name {
            "node" => NodeTest::AnyNode,
            "text" => NodeTest::Text,
            "comment" => NodeTest::Comment,
            "document-node" => NodeTest::Document,
            "processing-instruction" => match self.peek().cloned() {
                Some(Token::Name(None, target)) | Some(Token::String(target)) => {
                    self.position += 1;
                    NodeTest::ProcessingInstruction(Some(target))
                }
                _ => NodeTest::ProcessingInstruction(None),
            },
            "element" | "attribute" => {
                let attribute = name == "attribute";
                let name_test = match self.peek().cloned() {
                    Some(Token::Symbol("*")) => {
                        self.position += 1;
                        None
                    }
                    Some(Token::Name(prefix, local)) => {
                        self.position += 1;
                        Some(self.name_test(prefix.as_deref(), local)?)
                    }
                    _ => None,
                };
                if self.eat_symbol(",") {
                    return error("typed element()/attribute() tests are not supported");
                }
                if attribute {
                    NodeTest::Attribute(name_test)
                } else {
                    NodeTest::Element(name_test)
                }
            }
            other => return error(format!("`{other}()` tests are not supported")),
        };
        self.expect_symbol(")")?;
        Ok(test)
    }

    fn primary_expr(&mut self) -> Result<Expr> {
        match self.next() {
            Some(Token::String(value)) => Ok(Expr::Literal(Atomic::Str(value))),
            Some(Token::Integer(value)) => Ok(Expr::Literal(Atomic::Int(value))),
            Some(Token::Decimal(value)) => Ok(Expr::Literal(Atomic::Dec(value))),
            Some(Token::Double(value)) => Ok(Expr::Literal(Atomic::Dbl(value))),
            Some(Token::Variable(prefix, name)) => {
                if prefix.is_some() {
                    return error("prefixed variable names are not supported");
                }
                if !self.locals.contains(&name) && !self.context.variables.contains(&name) {
                    return error(format!("variable `${name}` is not declared"));
                }
                Ok(Expr::Var(name))
            }
            Some(Token::Symbol("(")) => {
                if self.eat_symbol(")") {
                    return Ok(Expr::Sequence(Vec::new()));
                }
                let expr = self.expr()?;
                self.expect_symbol(")")?;
                // `(a)[1]` filters the whole sequence: the predicates go on
                // a filter expression around it, unlike `a[1]`.
                Ok(expr)
            }
            Some(Token::Symbol(".")) => Ok(Expr::ContextItem),
            Some(Token::Name(prefix, local)) if self.at_symbol("(") => {
                self.position += 1;
                let args = self.arguments()?;
                self.function_call(prefix.as_deref(), &local, args)
            }
            Some(token) => error(format!("unexpected {}", describe(&token))),
            None => error("unexpected end of expression"),
        }
    }

    /// A function call's arguments, after the `(`.
    fn arguments(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if !self.eat_symbol(")") {
            loop {
                args.push(self.expr_single()?);
                if self.eat_symbol(")") {
                    break;
                }
                self.expect_symbol(",")?;
            }
        }
        Ok(args)
    }

    fn function_call(&self, prefix: Option<&str>, local: &str, args: Vec<Expr>) -> Result<Expr> {
        let namespace = match prefix {
            None => FN_NS,
            Some(prefix) => self.namespace(prefix)?,
        };
        if namespace == XS_NS {
            let target = AtomicType::from_xs_name(local)
                .filter(|target| !matches!(target, AtomicType::AnyAtomic | AtomicType::Numeric))
                .ok_or_else(|| {
                    CompileError(format!(
                        "constructor function xs:{local}() is not supported"
                    ))
                })?;
            let [arg] = <[Expr; 1]>::try_from(args)
                .map_err(|_| CompileError(format!("xs:{local}() takes one argument")))?;
            return Ok(Expr::Cast(Box::new(arg), target, true));
        }
        if namespace != FN_NS {
            return error(format!(
                "function {}:{local}() is not supported",
                prefix.unwrap_or_default()
            ));
        }
        let function = Function::from_name(local, args.len())?
            .with_static_base(self.context.base_uri.as_deref());
        let mut args = args;
        // Regular expressions given as literals are compiled once, here. An
        // invalid one stays a literal, and is an error when evaluated, as in
        // XPath; one this crate can't translate is unsupported.
        if let Some((pattern, flags)) = function.regex_arguments() {
            let literal = |expr: Option<&Expr>| match expr {
                Some(Expr::Literal(Atomic::Str(text))) => Some(text.clone()),
                None => Some(String::new()),
                _ => None,
            };
            if let (Some(regex), Some(flag_text)) =
                (literal(args.get(pattern)), literal(args.get(flags)))
            {
                match CompiledRegex::new(&regex, &flag_text) {
                    Ok(compiled) => {
                        args[pattern] = Expr::Call(
                            Function::CompiledRegex(std::sync::Arc::new(compiled)),
                            Vec::new(),
                        );
                    }
                    Err(RegexError::Invalid(..)) => {}
                    Err(RegexError::Unsupported(message)) => {
                        return error(format!("regular expression `{regex}`: {message}"));
                    }
                }
            }
        }
        Ok(Expr::Call(function, args))
    }

    // --- Patterns -------------------------------------------------------

    /// `p | p | …`, as the path patterns it matches.
    fn union_pattern(&mut self) -> Result<Vec<PathPattern>> {
        let mut alternatives = self.path_pattern()?;
        while self.eat_symbol("|") || self.eat_keyword("union") {
            alternatives.extend(self.path_pattern()?);
            if alternatives.len() > MAX_ALTERNATIVES {
                return error(format!(
                    "pattern has more than {MAX_ALTERNATIVES} alternatives"
                ));
            }
        }
        Ok(alternatives)
    }

    /// A path pattern. A parenthesised union used as a step is distributed
    /// over the path, so `(a | b)/c` gives the path patterns `a/c` and `b/c`.
    fn path_pattern(&mut self) -> Result<Vec<PathPattern>> {
        let mut rooted = false;
        let mut descendant = false;
        if self.eat_symbol("/") {
            rooted = true;
            if !self.at_step_start() && !self.at_symbol("(") {
                return Ok(vec![PathPattern {
                    rooted,
                    steps: Vec::new(),
                }]);
            }
        } else if self.eat_symbol("//") {
            descendant = true;
        }
        let mut paths = vec![PathPattern {
            rooted,
            steps: Vec::new(),
        }];
        loop {
            if self.eat_symbol("(") {
                let inner = self.parenthesised_pattern()?;
                let mut joined = Vec::new();
                for path in &paths {
                    for alternative in &inner {
                        let joins_a_path = !path.steps.is_empty() || rooted || descendant;
                        if joins_a_path && (alternative.rooted || alternative.steps[0].descendant) {
                            return error("a parenthesised pattern after `/` must be relative");
                        }
                        let mut steps = path.steps.clone();
                        let first = steps.len();
                        steps.extend(alternative.steps.iter().cloned());
                        steps[first].descendant |= descendant;
                        joined.push(PathPattern {
                            rooted: path.rooted || alternative.rooted,
                            steps,
                        });
                    }
                }
                if joined.len() > MAX_ALTERNATIVES {
                    return error(format!(
                        "pattern has more than {MAX_ALTERNATIVES} alternatives"
                    ));
                }
                paths = joined;
            } else {
                let step = self.axis_step()?.ok_or_else(|| {
                    CompileError(
                        "patterns may only contain axis steps and parenthesised patterns".into(),
                    )
                })?;
                if !matches!(step.axis, Axis::Child | Axis::Attribute) {
                    return error("patterns may only use the child and attribute axes");
                }
                let positional = step.predicates.iter().any(Expr::may_be_positional);
                for path in &mut paths {
                    path.steps.push(PatternStep {
                        descendant,
                        step: step.clone(),
                        positional,
                    });
                }
            }
            if self.eat_symbol("/") {
                descendant = false;
            } else if self.eat_symbol("//") {
                descendant = true;
            } else {
                break;
            }
        }
        Ok(paths)
    }

    /// `( pattern )` with any predicates, after the `(`. The predicates apply
    /// to each alternative's last step, which only gives the same nodes when
    /// they don't depend on positions.
    fn parenthesised_pattern(&mut self) -> Result<Vec<PathPattern>> {
        if self.depth == MAX_DEPTH {
            return error(format!(
                "expression nested more than {MAX_DEPTH} levels deep"
            ));
        }
        self.depth += 1;
        let inner = self.union_pattern();
        self.depth -= 1;
        let mut inner = inner?;
        self.expect_symbol(")")?;
        let predicates = self.predicates()?;
        if predicates.iter().any(Expr::may_be_positional) {
            return error("positional predicates on a parenthesised pattern are not supported");
        }
        for alternative in &mut inner {
            let Some(last) = alternative.steps.last_mut() else {
                return error("`/` in a parenthesised pattern is not supported");
            };
            last.step.predicates.extend(predicates.iter().cloned());
        }
        Ok(inner)
    }
}

fn is_kind_test(name: &str) -> bool {
    matches!(
        name,
        "node"
            | "text"
            | "comment"
            | "processing-instruction"
            | "element"
            | "attribute"
            | "document-node"
            | "schema-element"
            | "schema-attribute"
    )
}

fn descendant_or_self() -> Expr {
    Expr::Step(Step {
        axis: Axis::DescendantOrSelf,
        test: NodeTest::AnyNode,
        predicates: Vec::new(),
    })
}

/// `left/right`, turning `//name` (without predicates) into a single
/// descendant step.
fn join(left: Expr, right: Expr) -> Expr {
    if let Expr::Path(inner_left, inner_right) = &left
        && let Expr::Step(Step {
            axis: Axis::DescendantOrSelf,
            test: NodeTest::AnyNode,
            predicates,
        }) = inner_right.as_ref()
        && predicates.is_empty()
        && let Expr::Step(step) = &right
        && step.axis == Axis::Child
        && step.predicates.is_empty()
    {
        let mut step = step.clone();
        step.axis = Axis::Descendant;
        return Expr::Path(inner_left.clone(), Box::new(Expr::Step(step)));
    }
    Expr::Path(Box::new(left), Box::new(right))
}

fn describe(token: &Token) -> String {
    match token {
        Token::Name(Some(prefix), local) => format!("`{prefix}:{local}`"),
        Token::Name(None, local) => format!("`{local}`"),
        Token::PrefixWildcard(prefix) => format!("`{prefix}:*`"),
        Token::LocalWildcard(local) => format!("`*:{local}`"),
        Token::Variable(_, name) => format!("`${name}`"),
        Token::String(value) => format!("string `{value}`"),
        Token::Integer(value) => format!("`{value}`"),
        Token::Decimal(value) | Token::Double(value) => format!("`{value}`"),
        Token::Symbol(symbol) => format!("`{symbol}`"),
    }
}
