use std::path::Path;

use serde_json::Map;
use serde_json::Value;
use serde_json::from_slice;
use serde_json::json;
use serde_json::to_value;
use serde_json::to_vec_pretty;
use serde_yaml::from_slice as yaml_from_slice;
use serde_yaml::to_string as yaml_to_string;
use sha2::Digest;
use sha2::Sha256;
use thiserror::Error;
use toml::Value as TomlValue;
use toml::from_str as toml_from_str;
use toml::to_string_pretty as toml_to_string_pretty;

#[derive(Debug, Error)]
pub(crate) enum ConfigDocumentError {
    #[error("configuration file is not valid UTF-8")]
    InvalidUtf8,
    #[error("configuration file format is not supported")]
    UnsupportedFormat,
    #[error("configuration document is invalid: {0}")]
    InvalidDocument(String),
    #[error("configuration patch is invalid: {0}")]
    InvalidPatch(String),
    #[error("configuration patch requires explicit lossy confirmation")]
    LossyPatch,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConfigFormat {
    Properties,
    Json,
    Yaml,
    Toml,
}

struct PropertyLine {
    body: String,
    ending: String,
    key: Option<String>,
    value_start: usize,
    value_end: usize,
}

pub(crate) fn is_supported_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("properties")
                || extension.eq_ignore_ascii_case("json")
                || extension.eq_ignore_ascii_case("yaml")
                || extension.eq_ignore_ascii_case("yml")
                || extension.eq_ignore_ascii_case("toml")
        })
}

pub(crate) fn document_id(path: &str) -> String {
    sha256_hex(path.as_bytes())
}

pub(crate) fn document(path: &str, content: &[u8]) -> Result<Value, ConfigDocumentError> {
    match config_format(path) {
        Some(ConfigFormat::Properties) => properties_document(path, content),
        Some(format) => structured_document(path, content, format),
        None => Err(ConfigDocumentError::UnsupportedFormat),
    }
}

fn properties_document(path: &str, content: &[u8]) -> Result<Value, ConfigDocumentError> {
    let content = std::str::from_utf8(content).map_err(|_| ConfigDocumentError::InvalidUtf8)?;
    let lines = parse_property_lines(content);
    let server_properties = is_server_properties(path);
    let mut values = Map::new();
    let mut schema_properties = Map::new();
    let mut ui_properties = Map::new();
    let mut unmapped = Vec::new();

    for line in &lines {
        if let Some(key) = &line.key {
            let value = &line.body[line.value_start..line.value_end];
            let schema = server_properties
                .then(|| server_property_schema(key))
                .flatten()
                .unwrap_or_else(|| json!({ "type": "string", "title": key }));
            let ui_schema = server_properties
                .then(|| server_property_ui_schema(key))
                .flatten()
                .unwrap_or_else(|| json!({ "widget": "text" }));
            let value = if server_properties {
                server_property_value(key, value)
            } else {
                Value::String(value.to_owned())
            };
            values.insert(key.clone(), value);
            schema_properties.insert(key.clone(), schema);
            ui_properties.insert(key.clone(), ui_schema);
        } else if !line.body.trim().is_empty() {
            unmapped.push(line.body.clone());
        }
    }

    let content_hash = sha256_hex(content.as_bytes());
    Ok(json!({
        "documentId": document_id(path),
        "path": path,
        "format": "PROPERTIES",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": schema_properties,
            "additionalProperties": { "type": "string" },
        },
        "uiSchema": {
            "type": "object",
            "properties": ui_properties,
        },
        "values": Value::Object(values),
        "revision": content_hash,
        "contentHash": content_hash,
        "unmapped": unmapped,
        "lossy": false,
    }))
}

fn is_server_properties(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("server.properties"))
}

fn server_property_schema(key: &str) -> Option<Value> {
    if is_server_boolean_property(key) {
        Some(json!({ "type": "boolean", "title": key }))
    } else if is_server_integer_property(key) {
        Some(json!({ "type": "integer", "title": key }))
    } else if let Some(options) = server_property_options(key) {
        Some(json!({ "type": "string", "title": key, "enum": options }))
    } else if key == "rcon.password" {
        Some(json!({
            "type": "string",
            "title": key,
            "writeOnly": true
        }))
    } else {
        None
    }
}

fn server_property_ui_schema(key: &str) -> Option<Value> {
    if is_server_boolean_property(key) {
        Some(json!({ "widget": "checkbox" }))
    } else if is_server_integer_property(key) {
        Some(json!({ "widget": "number" }))
    } else if let Some(options) = server_property_options(key) {
        Some(json!({ "widget": "select", "options": options }))
    } else if key == "rcon.password" {
        Some(json!({ "widget": "password", "sensitive": true }))
    } else {
        None
    }
}

fn server_property_value(key: &str, value: &str) -> Value {
    if is_server_boolean_property(key) {
        value
            .parse::<bool>()
            .map(Value::Bool)
            .unwrap_or_else(|_| Value::String(value.to_owned()))
    } else if is_server_integer_property(key) {
        value
            .parse::<i64>()
            .map(|value| json!(value))
            .unwrap_or_else(|_| Value::String(value.to_owned()))
    } else {
        Value::String(value.to_owned())
    }
}

fn is_server_boolean_property(key: &str) -> bool {
    matches!(
        key,
        "allow-flight"
            | "allow-nether"
            | "broadcast-console-to-ops"
            | "broadcast-rcon-to-ops"
            | "enable-command-block"
            | "enable-jmx-monitoring"
            | "enable-query"
            | "enable-rcon"
            | "enable-status"
            | "enforce-secure-profile"
            | "enforce-whitelist"
            | "force-gamemode"
            | "hardcore"
            | "hide-online-players"
            | "online-mode"
            | "pvp"
            | "spawn-animals"
            | "spawn-monsters"
            | "spawn-npcs"
            | "sync-chunk-writes"
            | "use-native-transport"
            | "white-list"
    )
}

fn is_server_integer_property(key: &str) -> bool {
    matches!(
        key,
        "entity-broadcast-range-percentage"
            | "function-permission-level"
            | "max-players"
            | "max-tick-time"
            | "network-compression-threshold"
            | "op-permission-level"
            | "player-idle-timeout"
            | "query.port"
            | "rate-limit"
            | "rcon.port"
            | "server-port"
            | "simulation-distance"
            | "spawn-protection"
            | "view-distance"
    )
}

fn server_property_options(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "difficulty" => Some(&["peaceful", "easy", "normal", "hard"]),
        "gamemode" => Some(&["survival", "creative", "adventure", "spectator"]),
        _ => None,
    }
}

fn structured_document(
    path: &str,
    content: &[u8],
    format: ConfigFormat,
) -> Result<Value, ConfigDocumentError> {
    let content_value = parse_structured_value(content, format)?;
    let values = content_value.as_object().ok_or_else(|| {
        ConfigDocumentError::InvalidDocument(format.root_must_be_object_message().to_owned())
    })?;
    let mut schema_properties = Map::new();
    let mut ui_properties = Map::new();
    for (key, value) in values {
        schema_properties.insert(key.clone(), json_schema_for(value));
        ui_properties.insert(key.clone(), json_ui_schema_for(value));
    }

    let content_hash = sha256_hex(content);
    Ok(json!({
        "documentId": document_id(path),
        "path": path,
        "format": format.name(),
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": schema_properties,
            "additionalProperties": true,
        },
        "uiSchema": {
            "type": "object",
            "properties": ui_properties,
        },
        "values": content_value,
        "revision": content_hash,
        "contentHash": content_hash,
        "unmapped": [],
        "lossy": true,
    }))
}

pub(crate) fn summary(document: &Value) -> Value {
    json!({
        "documentId": document.get("documentId"),
        "path": document.get("path"),
        "format": document.get("format"),
        "revision": document.get("revision"),
        "contentHash": document.get("contentHash"),
        "lossy": document.get("lossy"),
    })
}

pub(crate) fn patch(
    path: &str,
    content: &[u8],
    patch: &Value,
    allow_lossy: bool,
) -> Result<Vec<u8>, ConfigDocumentError> {
    match config_format(path) {
        Some(ConfigFormat::Properties) => properties_patch(content, patch),
        Some(format) => structured_patch(content, patch, format, allow_lossy),
        None => Err(ConfigDocumentError::UnsupportedFormat),
    }
}

fn properties_patch(content: &[u8], patch: &Value) -> Result<Vec<u8>, ConfigDocumentError> {
    let content = std::str::from_utf8(content).map_err(|_| ConfigDocumentError::InvalidUtf8)?;
    let patch = patch.as_object().ok_or_else(|| {
        ConfigDocumentError::InvalidPatch("the merge patch must be an object".to_owned())
    })?;
    let mut lines = parse_property_lines(content);
    let mut appended = Vec::new();

    for (key, value) in patch {
        validate_property_key(key)?;
        match value {
            Value::Null => lines.retain(|line| line.key.as_deref() != Some(key.as_str())),
            value => {
                let value = property_value(value)?;
                if let Some(index) = lines
                    .iter()
                    .rposition(|line| line.key.as_deref() == Some(key.as_str()))
                {
                    let line = &mut lines[index];
                    line.body
                        .replace_range(line.value_start..line.value_end, &value);
                    line.value_end = line.value_start + value.len();
                } else {
                    appended.push((key.clone(), value));
                }
            }
        }
    }

    let newline = lines
        .iter()
        .find_map(|line| (!line.ending.is_empty()).then_some(line.ending.as_str()))
        .unwrap_or("\n")
        .to_owned();
    let had_final_newline = content.ends_with('\n');
    let mut output = render_property_lines(&lines);
    for (key, value) in appended {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(&newline);
        }
        output.push_str(&key);
        output.push('=');
        output.push_str(&value);
        if had_final_newline {
            output.push_str(&newline);
        }
    }

    Ok(output.into_bytes())
}

fn structured_patch(
    content: &[u8],
    patch: &Value,
    format: ConfigFormat,
    allow_lossy: bool,
) -> Result<Vec<u8>, ConfigDocumentError> {
    if !allow_lossy {
        return Err(ConfigDocumentError::LossyPatch);
    }
    let mut content_value = parse_structured_value(content, format)?;
    let Some(content_object) = content_value.as_object_mut() else {
        return Err(ConfigDocumentError::InvalidDocument(
            format.root_must_be_object_message().to_owned(),
        ));
    };
    let patch = patch.as_object().ok_or_else(|| {
        ConfigDocumentError::InvalidPatch("the merge patch must be an object".to_owned())
    })?;
    for (key, value) in patch {
        if value.is_null() {
            content_object.remove(key);
            continue;
        }
        let current = content_object.entry(key.clone()).or_insert(Value::Null);
        apply_merge_patch(current, value);
    }
    serialize_structured_value(&content_value, format)
}

fn apply_merge_patch(target: &mut Value, patch: &Value) {
    let Some(patch_object) = patch.as_object() else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Some(target_object) = target.as_object_mut() else {
        return;
    };
    for (key, value) in patch_object {
        if value.is_null() {
            target_object.remove(key);
        } else {
            let current = target_object.entry(key.clone()).or_insert(Value::Null);
            apply_merge_patch(current, value);
        }
    }
}

fn config_format(path: &str) -> Option<ConfigFormat> {
    let extension = path.rsplit_once('.')?.1;
    if extension.eq_ignore_ascii_case("properties") {
        Some(ConfigFormat::Properties)
    } else if extension.eq_ignore_ascii_case("json") {
        Some(ConfigFormat::Json)
    } else if extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml") {
        Some(ConfigFormat::Yaml)
    } else if extension.eq_ignore_ascii_case("toml") {
        Some(ConfigFormat::Toml)
    } else {
        None
    }
}

impl ConfigFormat {
    fn name(self) -> &'static str {
        match self {
            Self::Properties => "PROPERTIES",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
        }
    }

    fn root_must_be_object_message(self) -> &'static str {
        match self {
            Self::Properties => "properties configuration root must be an object",
            Self::Json => "JSON configuration root must be an object",
            Self::Yaml => "YAML configuration root must be an object",
            Self::Toml => "TOML configuration root must be an object",
        }
    }
}

fn parse_structured_value(
    content: &[u8],
    format: ConfigFormat,
) -> Result<Value, ConfigDocumentError> {
    match format {
        ConfigFormat::Json => from_slice(content)
            .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string())),
        ConfigFormat::Yaml => yaml_from_slice(content)
            .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string())),
        ConfigFormat::Toml => {
            let content =
                std::str::from_utf8(content).map_err(|_| ConfigDocumentError::InvalidUtf8)?;
            let value: TomlValue = toml_from_str(content)
                .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string()))?;
            to_value(value).map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string()))
        }
        ConfigFormat::Properties => Err(ConfigDocumentError::UnsupportedFormat),
    }
}

fn serialize_structured_value(
    value: &Value,
    format: ConfigFormat,
) -> Result<Vec<u8>, ConfigDocumentError> {
    match format {
        ConfigFormat::Json => {
            let mut output = to_vec_pretty(value)
                .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string()))?;
            output.push(b'\n');
            Ok(output)
        }
        ConfigFormat::Yaml => yaml_to_string(value)
            .map(String::into_bytes)
            .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string())),
        ConfigFormat::Toml => toml_to_string_pretty(value)
            .map(String::into_bytes)
            .map_err(|error| ConfigDocumentError::InvalidDocument(error.to_string())),
        ConfigFormat::Properties => Err(ConfigDocumentError::UnsupportedFormat),
    }
}

fn json_schema_for(value: &Value) -> Value {
    match value {
        Value::Null => json!({ "type": "null" }),
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Number(value) if value.is_i64() || value.is_u64() => {
            json!({ "type": "integer" })
        }
        Value::Number(_) => json!({ "type": "number" }),
        Value::String(_) => json!({ "type": "string" }),
        Value::Array(_) => json!({ "type": "array", "items": {} }),
        Value::Object(_) => json!({ "type": "object", "additionalProperties": true }),
    }
}

fn json_ui_schema_for(value: &Value) -> Value {
    let widget = match value {
        Value::Null => "text",
        Value::Bool(_) => "checkbox",
        Value::Number(_) => "number",
        Value::String(_) => "text",
        Value::Array(_) | Value::Object(_) => "json",
    };
    json!({ "widget": widget })
}

fn parse_property_lines(content: &str) -> Vec<PropertyLine> {
    if content.is_empty() {
        return Vec::new();
    }

    let bytes = content.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let body_end = if index > start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        lines.push(parse_property_line(
            &content[start..body_end],
            &content[body_end..index + 1],
        ));
        start = index + 1;
    }
    if start < content.len() {
        lines.push(parse_property_line(&content[start..], ""));
    }
    lines
}

fn parse_property_line(body: &str, ending: &str) -> PropertyLine {
    let leading = body.len() - body.trim_start_matches(char::is_whitespace).len();
    let logical = &body[leading..];
    if logical.is_empty() || logical.starts_with('#') || logical.starts_with('!') {
        return PropertyLine {
            body: body.to_owned(),
            ending: ending.to_owned(),
            key: None,
            value_start: 0,
            value_end: 0,
        };
    }

    let (key_end, value_start) = property_ranges(body, leading, logical);
    let key = unescape_property_key(body[leading..key_end].trim_end());
    if key.is_empty() {
        return PropertyLine {
            body: body.to_owned(),
            ending: ending.to_owned(),
            key: None,
            value_start: 0,
            value_end: 0,
        };
    }
    let value_end = body[..body.len()]
        .trim_end_matches(char::is_whitespace)
        .len()
        .max(value_start);

    PropertyLine {
        body: body.to_owned(),
        ending: ending.to_owned(),
        key: Some(key),
        value_start,
        value_end,
    }
}

fn property_ranges(body: &str, leading: usize, logical: &str) -> (usize, usize) {
    let mut escaped = false;
    for (offset, character) in logical.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '=' | ':') {
            let key_end = leading + offset;
            let mut value_start = leading + offset + character.len_utf8();
            while body[value_start..].starts_with(char::is_whitespace) {
                value_start += body[value_start..].chars().next().map_or(0, char::len_utf8);
            }
            return (key_end, value_start);
        }
        if character.is_whitespace() {
            let key_end = leading + offset;
            let mut value_start = leading + offset;
            while value_start < body.len()
                && body[value_start..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                value_start += body[value_start..].chars().next().map_or(0, char::len_utf8);
            }
            if body[value_start..].starts_with(['=', ':']) {
                value_start += 1;
                while value_start < body.len()
                    && body[value_start..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    value_start += body[value_start..].chars().next().map_or(0, char::len_utf8);
                }
            }
            return (key_end, value_start);
        }
    }
    (body.len(), body.len())
}

fn unescape_property_key(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            result.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'f' => '\u{000c}',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn render_property_lines(lines: &[PropertyLine]) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str(&line.body);
        output.push_str(&line.ending);
    }
    output
}

fn validate_property_key(key: &str) -> Result<(), ConfigDocumentError> {
    if key.is_empty()
        || key.chars().any(|character| {
            character.is_whitespace() || matches!(character, '=' | ':' | '\r' | '\n' | '\0')
        })
    {
        return Err(ConfigDocumentError::InvalidPatch(format!(
            "property key is invalid: {key}"
        )));
    }
    Ok(())
}

fn property_value(value: &Value) -> Result<String, ConfigDocumentError> {
    let value = match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => return Ok(String::new()),
        Value::Array(_) | Value::Object(_) => {
            return Err(ConfigDocumentError::InvalidPatch(
                "property values must be strings, booleans, or numbers".to_owned(),
            ));
        }
    };
    if value.contains(['\r', '\n', '\0']) {
        return Err(ConfigDocumentError::InvalidPatch(
            "property values cannot contain line breaks or NUL".to_owned(),
        ));
    }
    Ok(value)
}

fn sha256_hex(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;

    use super::ConfigDocumentError;
    use super::document;
    use super::patch;

    #[test]
    fn recognizes_properties_without_rewriting_comments_or_order() {
        let content = b"# keep this\r\nmotd = Welcome\r\nonline-mode=true\r\nunknown line\r\n";
        let document = document("server.properties", content).expect("properties are parsed");

        assert_eq!(document["format"], "PROPERTIES");
        assert_eq!(document["values"]["motd"], "Welcome");
        assert_eq!(document["values"]["online-mode"], true);
        assert_eq!(document["unmapped"], json!(["# keep this"]));
    }

    #[test]
    fn applies_server_properties_types_enums_and_sensitive_ui_metadata() {
        let document = document(
            "server.properties",
            b"online-mode=true\nserver-port=25565\ndifficulty=hard\nrcon.password=secret\n",
        )
        .expect("server.properties is parsed");

        assert_eq!(document["values"]["online-mode"], true);
        assert_eq!(document["values"]["server-port"], 25565);
        assert_eq!(
            document["schema"]["properties"]["server-port"]["type"],
            "integer"
        );
        assert_eq!(
            document["schema"]["properties"]["difficulty"]["enum"],
            json!(["peaceful", "easy", "normal", "hard"])
        );
        assert_eq!(
            document["uiSchema"]["properties"]["difficulty"]["widget"],
            "select"
        );
        assert_eq!(
            document["uiSchema"]["properties"]["rcon.password"]["sensitive"],
            true
        );
    }

    #[test]
    fn patches_values_and_preserves_property_layout() {
        let content = b"# keep this\nfirst=one\n\nsecond=two";
        let updated = patch(
            "server.properties",
            content,
            &json!({ "first": "changed", "second": null, "third": false }),
            false,
        )
        .expect("properties are patched");

        assert_eq!(updated, b"# keep this\nfirst=changed\n\nthird=false");
    }

    #[test]
    fn rejects_structured_property_values() {
        let error = patch(
            "server.properties",
            b"motd=MCNP",
            &json!({ "motd": [] }),
            false,
        )
        .expect_err("array values cannot be represented in properties");

        assert!(error.to_string().contains("strings"));
    }

    #[test]
    fn recognizes_json_and_builds_typed_schema() {
        let document = document(
            "config/settings.json",
            br#"{"enabled":true,"maxPlayers":12,"motd":"MCNP","nested":{"debug":false}}"#,
        )
        .expect("JSON configuration is parsed");

        assert_eq!(document["format"], "JSON");
        assert_eq!(document["values"]["enabled"], true);
        assert_eq!(
            document["schema"]["properties"]["maxPlayers"]["type"],
            "integer"
        );
        assert_eq!(
            document["uiSchema"]["properties"]["enabled"]["widget"],
            "checkbox"
        );
        assert_eq!(document["lossy"], true);
        assert_eq!(document["unmapped"], json!([]));
    }

    #[test]
    fn requires_lossy_confirmation_for_json_merge_patch() {
        let content = br#"{"enabled":true,"nested":{"debug":false},"removed":"value"}"#;
        let patch_value = json!({
            "enabled": false,
            "nested": { "debug": true, "level": 2 },
            "removed": null,
        });
        let error = patch("settings.json", content, &patch_value, false)
            .expect_err("JSON formatting requires explicit lossy confirmation");
        assert!(matches!(error, ConfigDocumentError::LossyPatch));

        let updated = patch("settings.json", content, &patch_value, true)
            .expect("JSON merge patch is applied");
        let updated: Value = from_slice(&updated).expect("patched JSON remains valid");
        assert_eq!(updated["enabled"], false);
        assert_eq!(updated["nested"]["debug"], true);
        assert_eq!(updated["nested"]["level"], 2);
        assert!(updated.get("removed").is_none());
    }

    #[test]
    fn recognizes_yaml_and_toml_and_normalizes_lossy_patches() {
        let cases: [(&str, &[u8]); 2] = [
            ("settings.yml", b"enabled: true\nnested:\n  debug: false\n"),
            (
                "settings.toml",
                b"enabled = true\n\n[nested]\ndebug = false\n",
            ),
        ];

        for (path, content) in cases {
            let parsed_document =
                document(path, content).expect("structured configuration is parsed");
            assert_eq!(
                parsed_document["format"],
                if path.ends_with("yml") {
                    "YAML"
                } else {
                    "TOML"
                }
            );
            assert_eq!(parsed_document["values"]["enabled"], true);
            assert_eq!(parsed_document["values"]["nested"]["debug"], false);
            assert_eq!(parsed_document["lossy"], true);

            let patch_value = json!({ "enabled": false, "nested": { "debug": true } });
            assert!(matches!(
                patch(path, content, &patch_value, false),
                Err(ConfigDocumentError::LossyPatch)
            ));
            let updated = patch(path, content, &patch_value, true)
                .expect("structured configuration patch is applied");
            let updated = document(path, &updated).expect("normalized configuration remains valid");
            assert_eq!(updated["values"]["enabled"], false);
            assert_eq!(updated["values"]["nested"]["debug"], true);
        }
    }
}
