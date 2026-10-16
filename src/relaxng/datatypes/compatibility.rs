//! The RELAX NG DTD Compatibility datatype library
//! (`http://relaxng.org/ns/compatibility/datatypes/1.0`): `ID`, `IDREF` and
//! `IDREFS`, with the same values as XSD's types of those names and no
//! parameters. What makes them IDs (uniqueness, references resolving) is
//! checked separately, as for XSD's (see `relaxng::idtypes`).

use super::registry::{DatatypeContext, DatatypeLibrary};
use super::xsd::XsdLibrary;

/// The namespace URI of the DTD Compatibility datatype library.
pub const COMPATIBILITY_DATATYPE_LIBRARY: &str =
    "http://relaxng.org/ns/compatibility/datatypes/1.0";

pub(crate) struct CompatibilityLibrary;

impl DatatypeLibrary for CompatibilityLibrary {
    fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String> {
        if !matches!(type_name, "ID" | "IDREF" | "IDREFS") {
            return Err(format!("unknown type `{type_name}`"));
        }
        if let Some((name, _)) = params.first() {
            return Err(format!(
                "`{type_name}` has no parameters, not even `{name}`"
            ));
        }
        Ok(())
    }

    fn matches(
        &self,
        type_name: &str,
        _params: &[(&str, &str)],
        value: &str,
        context: &DatatypeContext,
    ) -> bool {
        XsdLibrary.matches(type_name, &[], value, context)
    }

    fn values_equal(
        &self,
        type_name: &str,
        expected: &str,
        expected_context: &DatatypeContext,
        actual: &str,
        actual_context: &DatatypeContext,
    ) -> bool {
        XsdLibrary.values_equal(
            type_name,
            expected,
            expected_context,
            actual,
            actual_context,
        )
    }

    fn is_context_dependent(&self, _type_name: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_ncnames() {
        let context = DatatypeContext::empty();
        let library = CompatibilityLibrary;
        assert!(library.matches("ID", &[], " a1 ", &context));
        assert!(!library.matches("IDREF", &[], "1a", &context));
        assert!(library.matches("IDREFS", &[], "a b", &context));
        assert!(library.validate_params("ID", &[]).is_ok());
        assert!(library.validate_params("ID", &[("length", "1")]).is_err());
        assert!(library.validate_params("NCName", &[]).is_err());
    }
}
