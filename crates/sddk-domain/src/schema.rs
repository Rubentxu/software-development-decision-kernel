//! Runtime JSON Schema validation for structured SDDK results.

use serde_json::Value;
use thiserror::Error;

/// Errors emitted while validating an instance against a JSON Schema.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// The schema document could not be compiled.
    #[error("invalid JSON Schema: {0}")]
    Compile(String),
    /// A local `$ref` could not be resolved.
    #[error("cannot resolve local schema reference {reference:?}")]
    UnresolvedRef {
        /// Unresolved reference target.
        reference: String,
    },
}

/// Inlines local `$ref` targets before schema compilation.
///
/// References that are not JSON pointers (`#...`) or absolute URLs are treated
/// as sibling schema files and replaced by the document the loader returns,
/// recursively. This keeps validation offline without a resolver feature.
pub fn dereference_local_refs(
    schema: &Value,
    loader: &mut dyn FnMut(&str) -> Result<Value, SchemaError>,
) -> Result<Value, SchemaError> {
    match schema {
        Value::Object(mapping) => {
            let mut resolved = serde_json::Map::new();
            for (key, value) in mapping {
                if key == "$ref"
                    && let Some(target) = value
                        .as_str()
                        .filter(|target| !target.starts_with('#') && !target.contains("://"))
                {
                    let document = loader(target)?;
                    let inlined = dereference_local_refs(&document, loader)?;
                    if let Value::Object(fields) = inlined {
                        for (field, field_value) in fields {
                            resolved.insert(field, field_value);
                        }
                    }
                    continue;
                }
                resolved.insert(key.clone(), dereference_local_refs(value, loader)?);
            }
            Ok(Value::Object(resolved))
        }
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .map(|item| dereference_local_refs(item, loader))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

/// Validates an instance against a schema document.
///
/// Returns a list of human-readable validation errors; an empty list means the
/// instance conforms to the schema.
pub fn validate_against_schema(
    instance: &Value,
    schema: &Value,
) -> Result<Vec<String>, SchemaError> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| SchemaError::Compile(error.to_string()))?;
    Ok(validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect())
}

/// Validates an instance against a schema document loaded from a JSON string.
pub fn validate_against_schema_str(
    instance: &Value,
    schema_json: &str,
) -> Result<Vec<String>, SchemaError> {
    let schema: Value = serde_json::from_str(schema_json)
        .map_err(|error| SchemaError::Compile(format!("schema is not valid JSON: {error}")))?;
    validate_against_schema(instance, &schema)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_against_schema_str;

    const SCHEMA: &str = r#"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["id", "name"],
        "properties": {
            "id": {"type": "integer"},
            "name": {"type": "string", "minLength": 1}
        }
    }"#;

    #[test]
    fn accepts_conforming_instance() {
        let errors = validate_against_schema_str(&json!({"id": 1, "name": "x"}), SCHEMA).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn reports_concrete_validation_errors() {
        let errors =
            validate_against_schema_str(&json!({"id": "not-an-int", "name": ""}), SCHEMA).unwrap();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|error| error.contains("not-an-int")));
        assert!(errors.iter().any(|error| error.contains("shorter than 1")));
    }

    #[test]
    fn rejects_invalid_schema_document() {
        let result = validate_against_schema_str(&json!({}), "{not json");
        assert!(result.is_err());
    }
}
