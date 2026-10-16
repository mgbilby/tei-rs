//! ID, IDREF and IDREFS checking, as the RELAX NG DTD Compatibility
//! specification (section 4) defines it and Jing implements it.
//!
//! A schema gives an attribute an ID-type when its value is a `data` or
//! `value` pattern of datatype `ID`, `IDREF` or `IDREFS`, from the DTD
//! compatibility datatype library or XML Schema's. Such a schema must be
//! *compatible*: the ID-type of every attribute must follow from the names
//! of the attribute and its element alone, so documents can be checked
//! without validating them. [`IdTypes::analyze`] checks that and builds the
//! table; a document then has every ID value unique and every IDREF naming
//! one of them.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::relaxng::grammar::{NameClass, Pattern};
use crate::relaxng::simplify::{CompiledSchema, name_class_matches};

const COMPATIBILITY_LIBRARY: &str = "http://relaxng.org/ns/compatibility/datatypes/1.0";
const XSD_LIBRARY: &str = "http://www.w3.org/2001/XMLSchema-datatypes";

/// The ID-type of an attribute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdType {
    /// A unique identifier.
    Id,
    /// A reference to an identifier.
    Idref,
    /// Whitespace-separated references to identifiers.
    Idrefs,
}

/// Which attributes of which elements have an ID-type in a schema.
///
/// ```
/// use teixml::relaxng::{IdType, ResolveError, Schema, SchemaResolver, SchemaSource, SchemaSyntax};
/// # struct NoIncludes;
/// # impl SchemaResolver for NoIncludes {
/// #     fn resolve(&self, href: &str, _: &str) -> Result<SchemaSource, ResolveError> {
/// #         Err(ResolveError::new(href))
/// #     }
/// # }
///
/// let source = SchemaSource::new(
///     "element doc { element p { attribute xml:id { xsd:ID }?, attribute corresp { xsd:IDREFS }? }* }",
///     "doc.rnc",
///     SchemaSyntax::Compact,
/// );
/// let ids = Schema::compile(&source, &NoIncludes)?.id_types().clone();
/// const XML: &str = "http://www.w3.org/XML/1998/namespace";
/// assert_eq!(ids.get(("", "p"), (XML, "id")), Some(IdType::Id));
/// assert_eq!(ids.get(("", "p"), ("", "corresp")), Some(IdType::Idrefs));
/// assert_eq!(ids.get(("", "doc"), (XML, "id")), None);
/// # Ok::<(), teixml::relaxng::CompileError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct IdTypes {
    /// Attribute local names with an ID-type somewhere, to rule out most
    /// attributes without building a key.
    attribute_names: HashSet<String>,
    /// `(element namespace, element local, attribute namespace, attribute
    /// local)` → ID-type, with namespaces as `""` for none.
    types: HashMap<(String, String, String, String), IdType>,
    /// The same, for lookups without building owned keys: attribute local
    /// name → attribute namespace → element local name → element
    /// namespace.
    index: Nested<Nested<Nested<HashMap<String, IdType>>>>,
}

type Nested<T> = HashMap<String, T>;

impl IdTypes {
    /// Whether no attribute has an ID-type, so there is nothing to check.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// The ID-type of attribute `attribute` on element `element`, both as
    /// (namespace, local name) with `""` for no namespace.
    pub fn get(&self, element: (&str, &str), attribute: (&str, &str)) -> Option<IdType> {
        self.index
            .get(attribute.1)?
            .get(attribute.0)?
            .get(element.1)?
            .get(element.0)
            .copied()
    }

    /// Builds the table for `schema`, or explains why the schema isn't
    /// compatible (with Jing's wording).
    pub(crate) fn analyze(schema: &CompiledSchema) -> Result<Self, String> {
        let mut elements = Vec::new();
        let mut visited = HashSet::new();
        find_elements(&schema.start, schema, &mut visited, &mut elements)?;
        for (name, pattern) in &schema.definitions {
            if visited.insert(name.as_str()) {
                find_elements(pattern, schema, &mut visited, &mut elements)?;
            }
        }

        // Every attribute each element pattern can have, with its ID-type.
        let mut analysed = Vec::new();
        let mut cache = HashMap::new();
        for (element_name, body) in elements {
            let mut attributes = Vec::new();
            for pattern in body {
                collect_attributes(pattern, schema, &mut cache, &mut attributes)?;
            }
            // The same attribute pattern can be reached along several paths.
            let mut seen = HashSet::new();
            attributes.retain(|attribute| seen.insert(std::ptr::from_ref(attribute.0)));
            analysed.push((element_name, attributes));
        }

        let mut table = IdTypes::default();
        for (element_name, attributes) in &analysed {
            for &(attribute_name, id_type) in attributes {
                let Some(id_type) = id_type else { continue };
                let (Some(elements), Some(attribute_names)) =
                    (simple_names(element_name), simple_names(attribute_name))
                else {
                    return Err(if simple_names(element_name).is_none() {
                        r#"an "element" pattern containing an "attribute" pattern with a non-null ID-type must have a name class that contains only "choice" and "name" elements"#.into()
                    } else {
                        r#"an "attribute" pattern with a non-null ID-type must have a name class that contains only "choice" and "name" elements"#.into()
                    });
                };
                for (element_ns, element_local) in &elements {
                    for (attribute_ns, attribute_local) in &attribute_names {
                        let key = (
                            element_ns.clone(),
                            element_local.clone(),
                            attribute_ns.clone(),
                            attribute_local.clone(),
                        );
                        match table.types.insert(key, id_type) {
                            Some(previous) if previous != id_type => {
                                return Err(conflict(attribute_local, element_local));
                            }
                            _ => {}
                        }
                        table.attribute_names.insert(attribute_local.clone());
                    }
                }
            }
        }

        // An attribute that one element pattern gives an ID-type must have
        // it in every element pattern that allows that element and
        // attribute name. Typed pairs are indexed by attribute name, since
        // few attribute names have an ID-type.
        let mut typed: ByAttribute<'_> = HashMap::new();
        for (element_ns, element_local, attribute_ns, attribute_local) in table.types.keys() {
            typed
                .entry((Some(attribute_ns.clone()), attribute_local.clone()))
                .or_default()
                .push((Some(element_ns.clone()), element_local.as_str()));
        }
        for (element_name, attributes) in &analysed {
            for &(attribute_name, id_type) in attributes {
                if id_type.is_some() {
                    continue;
                }
                // Most attributes are named, with a name no ID-typed
                // attribute has.
                if let NameClass::Name { lexical, .. } = attribute_name
                    && !table.attribute_names.contains(lexical)
                {
                    continue;
                }
                let candidates: Vec<_> = match simple_names(attribute_name) {
                    Some(names) => names
                        .into_iter()
                        .filter_map(|(ns, local)| typed.get_key_value(&(Some(ns), local)))
                        .collect(),
                    None => typed
                        .iter()
                        .filter(|((ns, local), _)| name_class_matches(attribute_name, ns, local))
                        .collect(),
                };
                for ((_, attribute_local), elements) in candidates {
                    if let Some((_, element_local)) = elements
                        .iter()
                        .find(|(ns, local)| name_class_matches(element_name, ns, local))
                    {
                        return Err(conflict(attribute_local, element_local));
                    }
                }
            }
        }
        for ((element_ns, element_local, attribute_ns, attribute_local), &id_type) in &table.types {
            table
                .index
                .entry(attribute_local.clone())
                .or_default()
                .entry(attribute_ns.clone())
                .or_default()
                .entry(element_local.clone())
                .or_default()
                .insert(element_ns.clone(), id_type);
        }
        Ok(table)
    }
}

fn conflict(attribute: &str, element: &str) -> String {
    format!(r#"conflicting ID-types for attribute "{attribute}" of element "{element}""#)
}

/// The ID-type a `data` or `value` pattern gives, if any.
fn data_id_type(pattern: &Pattern) -> Option<IdType> {
    let (datatype, library) = match pattern {
        Pattern::Data {
            datatype,
            datatype_library,
            ..
        } => (datatype.as_str(), datatype_library.as_deref()),
        Pattern::Value {
            datatype: Some(datatype),
            datatype_library,
            ..
        } => (datatype.as_str(), datatype_library.as_deref()),
        _ => return None,
    };
    if !matches!(library, Some(COMPATIBILITY_LIBRARY | XSD_LIBRARY)) {
        return None;
    }
    match datatype {
        "ID" => Some(IdType::Id),
        "IDREF" => Some(IdType::Idref),
        "IDREFS" => Some(IdType::Idrefs),
        _ => None,
    }
}

/// Every `element` pattern in `pattern`, including nested ones, following
/// `ref`s to definitions not visited yet.
fn find_elements<'s>(
    pattern: &'s Pattern,
    schema: &'s CompiledSchema,
    visited: &mut HashSet<&'s str>,
    out: &mut Vec<(&'s NameClass, &'s [Pattern])>,
) -> Result<(), String> {
    match pattern {
        Pattern::Element { name, body, .. } => {
            out.push((name, body));
            for child in body {
                find_elements(child, schema, visited, out)?;
            }
        }
        Pattern::Attribute { body, .. } => {
            for child in body {
                find_elements(child, schema, visited, out)?;
            }
        }
        Pattern::Group(children)
        | Pattern::Interleave(children)
        | Pattern::Choice(children)
        | Pattern::Optional(children)
        | Pattern::ZeroOrMore(children)
        | Pattern::OneOrMore(children)
        | Pattern::List(children)
        | Pattern::Mixed(children) => {
            for child in children {
                find_elements(child, schema, visited, out)?;
            }
        }
        Pattern::Data { except, .. } => {
            for child in except {
                find_elements(child, schema, visited, out)?;
            }
        }
        Pattern::Ref(name) | Pattern::ParentRef(name) => {
            if let Some((name, target)) = schema.definitions.get_key_value(name)
                && visited.insert(name.as_str())
            {
                find_elements(target, schema, visited, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Element names, as (namespace, local name), indexed by an attribute name.
type ByAttribute<'s> = HashMap<(Option<String>, String), Vec<(Option<String>, &'s str)>>;

/// An attribute pattern's name class and the ID-type it gives.
type Attribute<'s> = (&'s NameClass, Option<IdType>);

/// The attributes an element's content can have (not counting nested
/// elements'), each with its ID-type. Each definition's attributes are
/// worked out once and cached. Also rejects ID-typed `data` or `value`
/// patterns anywhere but as an attribute's whole value.
fn collect_attributes<'s>(
    pattern: &'s Pattern,
    schema: &'s CompiledSchema,
    cache: &mut HashMap<&'s str, std::rc::Rc<Vec<Attribute<'s>>>>,
    out: &mut Vec<Attribute<'s>>,
) -> Result<(), String> {
    match pattern {
        Pattern::Element { .. } => {}
        Pattern::Attribute { name, body, .. } => {
            let value = match body.as_slice() {
                [only] => resolve(only, schema),
                _ => None,
            };
            let id_type = value.and_then(data_id_type);
            if id_type.is_none() {
                for child in body {
                    reject_id_data(child, schema, &mut BTreeSet::new())?;
                }
            }
            out.push((name, id_type));
        }
        Pattern::Group(children)
        | Pattern::Interleave(children)
        | Pattern::Choice(children)
        | Pattern::Optional(children)
        | Pattern::ZeroOrMore(children)
        | Pattern::OneOrMore(children)
        | Pattern::Mixed(children) => {
            for child in children {
                collect_attributes(child, schema, cache, out)?;
            }
        }
        Pattern::List(_) | Pattern::Data { .. } | Pattern::Value { .. } => {
            reject_id_data(pattern, schema, &mut BTreeSet::new())?;
        }
        Pattern::Ref(name) | Pattern::ParentRef(name) => {
            if let Some((name, target)) = schema.definitions.get_key_value(name) {
                if let Some(attributes) = cache.get(name.as_str()) {
                    out.extend(attributes.iter().copied());
                } else {
                    let mut attributes = Vec::new();
                    collect_attributes(target, schema, cache, &mut attributes)?;
                    let mut seen = HashSet::new();
                    attributes.retain(|attribute| seen.insert(std::ptr::from_ref(attribute.0)));
                    out.extend(attributes.iter().copied());
                    cache.insert(name.as_str(), std::rc::Rc::new(attributes));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Follows `ref`s (and single-child wrappers) to the pattern an attribute's
/// value really is.
fn resolve<'s>(pattern: &'s Pattern, schema: &'s CompiledSchema) -> Option<&'s Pattern> {
    let mut pattern = pattern;
    for _ in 0..64 {
        match pattern {
            Pattern::Ref(name) | Pattern::ParentRef(name) => {
                pattern = schema.definitions.get(name)?
            }
            Pattern::Group(children)
            | Pattern::Interleave(children)
            | Pattern::Choice(children)
                if children.len() == 1 =>
            {
                pattern = &children[0];
            }
            _ => return Some(pattern),
        }
    }
    None
}

/// Fails if `pattern` contains an ID-typed `data` or `value` pattern that
/// isn't an attribute's whole value.
fn reject_id_data(
    pattern: &Pattern,
    schema: &CompiledSchema,
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    if data_id_type(pattern).is_some() {
        return Err(r#"a "data" or "value" pattern with non-null ID-type must occur as the child of an "attribute" pattern"#.into());
    }
    match pattern {
        Pattern::Group(children)
        | Pattern::Interleave(children)
        | Pattern::Choice(children)
        | Pattern::Optional(children)
        | Pattern::ZeroOrMore(children)
        | Pattern::OneOrMore(children)
        | Pattern::List(children)
        | Pattern::Mixed(children) => {
            for child in children {
                reject_id_data(child, schema, seen)?;
            }
        }
        Pattern::Data { except, .. } => {
            for child in except {
                reject_id_data(child, schema, seen)?;
            }
        }
        Pattern::Ref(name) | Pattern::ParentRef(name) => {
            if seen.insert(name.clone())
                && let Some(target) = schema.definitions.get(name)
                && !matches!(target, Pattern::Element { .. })
            {
                reject_id_data(target, schema, seen)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The names a name class made only of `name` and `choice` allows, as
/// (namespace, local name); `None` for any other name class.
fn simple_names(class: &NameClass) -> Option<Vec<(String, String)>> {
    match class {
        NameClass::Name {
            lexical, namespace, ..
        } => Some(vec![(
            namespace.clone().unwrap_or_default(),
            lexical.clone(),
        )]),
        NameClass::Choice(options) => {
            let mut names = Vec::new();
            for option in options {
                names.extend(simple_names(option)?);
            }
            Some(names)
        }
        _ => None,
    }
}
