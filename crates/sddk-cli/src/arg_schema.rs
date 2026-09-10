//! `arg_schema` — derive JSON Schema (draft-07) from `ArgSpec` (M7.4).
//!
//! Closes the deferred gap from M7.2 (`surface_integration` left
//! `parameters` / `input_schema` as empty object schemas pending this
//! derivation). `CommandSpec::args` is already a typed contract
//! (`ArgSpec { name, kind, value_type, required, default, help }`); this
//! module projects that contract into the JSON Schema dialect consumed
//! by OpenAI function-calling and Anthropic Tools.
//!
//! ## Mapping
//!
//! | `ArgKind` | JSON Schema shape |
//! |---|---|
//! | `Flag` | `{ type: "boolean", default: <default or false> }` |
//! | `Option` | `{ type: <mapped from ValueType>, default: <default or null> }` |
//! | `Positional` | `{ type: "string" }` |
//!
//! | `ValueType` | JSON Schema type |
//! |---|---|
//! | `String` | `"string"` |
//! | `Int` | `"integer"` |
//! | `Bool` | `"boolean"` |
//! | `Path` | `"string", format: "path"` |
//! | `Enum(vs)` | `"string", enum: <vs>` |
//!
//! Required args populate the top-level `required` array. Each property
//! carries the `description` from `ArgSpec::help`.

use crate::command_spec::{ArgKind, ArgSpec, CommandSpec, ValueType};
use serde_json::{Value, json};

/// Build a JSON Schema (draft-07) object for the given `args` slice.
///
/// Output shape:
/// ```json
/// {
///   "type": "object",
///   "properties": {
///     "<name>": { "type": "<string|integer|boolean>", "description": "...", "default": ... }
///   },
///   "required": ["<name>", ...]
/// }
/// ```
///
/// Returns `{ "type": "object", "properties": {}, "required": [] }` for
/// an empty `args` slice (preserves the M7.2 placeholder contract for
/// commands that haven't declared typed args).
pub fn arg_specs_to_json_schema(args: &[ArgSpec]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<String> = Vec::new();

    for arg in args {
        properties.insert(arg.name.clone(), arg_to_property(arg));
        if arg.required {
            required.push(arg.name.clone());
        }
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// Build a JSON Schema for an entire `CommandSpec`, with `$id`, `title`,
/// and `description` fields populated from the command's metadata.
///
/// Output shape:
/// ```json
/// {
///   "$id": "sddk://commands/<name>",
///   "$schema": "http://json-schema.org/draft-07/schema#",
///   "title": "<name>",
///   "description": "<about>",
///   "type": "object",
///   "properties": {...},
///   "required": [...]
/// }
/// ```
pub fn command_spec_to_json_schema(spec: &CommandSpec) -> Value {
    let mut schema = arg_specs_to_json_schema(&spec.args);
    if let Value::Object(ref mut map) = schema {
        map.insert(
            "$id".to_string(),
            json!(format!("sddk://commands/{}", spec.name)),
        );
        map.insert(
            "$schema".to_string(),
            json!("http://json-schema.org/draft-07/schema#"),
        );
        map.insert("title".to_string(), json!(spec.name));
        map.insert("description".to_string(), json!(spec.about));
    }
    schema
}

fn arg_to_property(arg: &ArgSpec) -> Value {
    let mut prop = serde_json::Map::new();
    match arg.kind {
        ArgKind::Flag => {
            prop.insert("type".to_string(), json!("boolean"));
            prop.insert(
                "default".to_string(),
                json!(arg.default.as_deref() == Some("true")),
            );
        }
        ArgKind::Option => {
            apply_value_type(&mut prop, &arg.value_type);
            if let Some(d) = &arg.default {
                prop.insert("default".to_string(), json!(d));
            } else {
                prop.insert("default".to_string(), Value::Null);
            }
        }
        ArgKind::Positional => {
            prop.insert("type".to_string(), json!("string"));
        }
    }
    prop.insert("description".to_string(), json!(arg.help));
    Value::Object(prop)
}

fn apply_value_type(prop: &mut serde_json::Map<String, Value>, vt: &ValueType) {
    match vt {
        ValueType::String => {
            prop.insert("type".to_string(), json!("string"));
        }
        ValueType::Int => {
            prop.insert("type".to_string(), json!("integer"));
        }
        ValueType::Bool => {
            prop.insert("type".to_string(), json!("boolean"));
        }
        ValueType::Path => {
            prop.insert("type".to_string(), json!("string"));
            prop.insert("format".to_string(), json!("path"));
        }
        ValueType::Enum(values) => {
            prop.insert("type".to_string(), json!("string"));
            prop.insert("enum".to_string(), json!(values));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::ArgSpec;

    fn make_arg(name: &str, kind: ArgKind, value_type: ValueType, required: bool) -> ArgSpec {
        ArgSpec {
            name: name.to_string(),
            kind,
            value_type,
            required,
            default: None,
            help: format!("{name} help"),
        }
    }

    #[test]
    fn empty_args_yields_empty_object_schema() {
        let schema = arg_specs_to_json_schema(&[]);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], json!({}));
        assert_eq!(schema["required"], json!([]));
    }

    #[test]
    fn string_option_appears_as_string_property() {
        let arg = make_arg("format", ArgKind::Option, ValueType::String, false);
        let schema = arg_specs_to_json_schema(&[arg]);
        let prop = &schema["properties"]["format"];
        assert_eq!(prop["type"], "string");
        assert_eq!(prop["description"], "format help");
        assert!(schema["required"].as_array().unwrap().is_empty());
    }

    #[test]
    fn int_flag_required_appears_in_required_array() {
        // ArgKind::Flag maps to boolean regardless of value_type, but
        // the required flag still flows into the top-level required array.
        let arg = make_arg("verbose", ArgKind::Flag, ValueType::Bool, true);
        let schema = arg_specs_to_json_schema(&[arg]);
        assert_eq!(schema["properties"]["verbose"]["type"], "boolean");
        assert_eq!(schema["required"], json!(["verbose"]));
    }

    #[test]
    fn bool_flag_default_true_sets_default() {
        let mut arg = make_arg("dry_run", ArgKind::Flag, ValueType::Bool, false);
        arg.default = Some("true".to_string());
        let schema = arg_specs_to_json_schema(&[arg]);
        assert_eq!(schema["properties"]["dry_run"]["default"], json!(true));
    }

    #[test]
    fn enum_value_type_emits_enum_constraint() {
        let arg = make_arg(
            "level",
            ArgKind::Option,
            ValueType::Enum(vec!["low".to_string(), "high".to_string()]),
            true,
        );
        let schema = arg_specs_to_json_schema(&[arg]);
        let prop = &schema["properties"]["level"];
        assert_eq!(prop["type"], "string");
        assert_eq!(prop["enum"], json!(["low", "high"]));
        assert_eq!(schema["required"], json!(["level"]));
    }

    #[test]
    fn path_value_type_includes_format_path() {
        let arg = make_arg("root", ArgKind::Option, ValueType::Path, false);
        let schema = arg_specs_to_json_schema(&[arg]);
        assert_eq!(schema["properties"]["root"]["type"], "string");
        assert_eq!(schema["properties"]["root"]["format"], "path");
    }

    #[test]
    fn command_spec_to_json_schema_includes_id_title_description() {
        let spec = CommandSpec::new("demo", "demo command").with_args(vec![make_arg(
            "name",
            ArgKind::Option,
            ValueType::String,
            true,
        )]);
        let schema = command_spec_to_json_schema(&spec);
        assert_eq!(schema["$id"], "sddk://commands/demo");
        assert_eq!(schema["title"], "demo");
        assert_eq!(schema["description"], "demo command");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["required"], json!(["name"]));
    }
}
