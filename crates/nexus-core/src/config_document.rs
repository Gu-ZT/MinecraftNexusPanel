use std::path::Path;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ConfigDocumentError {
    #[error("configuration file is not valid UTF-8")]
    InvalidUtf8,
    #[error("configuration file format is not supported")]
    UnsupportedFormat,
    #[error("configuration patch is invalid: {0}")]
    InvalidPatch(String),
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
        .is_some_and(|extension| extension.eq_ignore_ascii_case("properties"))
}

pub(crate) fn document_id(path: &str) -> String {
    sha256_hex(path.as_bytes())
}

pub(crate) fn document(path: &str, content: &[u8]) -> Result<Value, ConfigDocumentError> {
    if !path
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("properties"))
    {
        return Err(ConfigDocumentError::UnsupportedFormat);
    }
    let content = std::str::from_utf8(content).map_err(|_| ConfigDocumentError::InvalidUtf8)?;
    let lines = parse_property_lines(content);
    let mut values = Map::new();
    let mut schema_properties = Map::new();
    let mut ui_properties = Map::new();
    let mut unmapped = Vec::new();

    for line in &lines {
        if let Some(key) = &line.key {
            let value = &line.body[line.value_start..line.value_end];
            values.insert(key.clone(), Value::String(value.to_owned()));
            schema_properties.insert(
                key.clone(),
                json!({
                    "type": "string",
                    "title": key,
                }),
            );
            ui_properties.insert(key.clone(), json!({ "widget": "text" }));
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
) -> Result<Vec<u8>, ConfigDocumentError> {
    if !path
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("properties"))
    {
        return Err(ConfigDocumentError::UnsupportedFormat);
    }
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
    use serde_json::json;

    use super::document;
    use super::patch;

    #[test]
    fn recognizes_properties_without_rewriting_comments_or_order() {
        let content = b"# keep this\r\nmotd = Welcome\r\nonline-mode=true\r\nunknown line\r\n";
        let document = document("server.properties", content).expect("properties are parsed");

        assert_eq!(document["format"], "PROPERTIES");
        assert_eq!(document["values"]["motd"], "Welcome");
        assert_eq!(document["values"]["online-mode"], "true");
        assert_eq!(document["unmapped"], json!(["# keep this"]));
    }

    #[test]
    fn patches_values_and_preserves_property_layout() {
        let content = b"# keep this\nfirst=one\n\nsecond=two";
        let updated = patch(
            "server.properties",
            content,
            &json!({ "first": "changed", "second": null, "third": false }),
        )
        .expect("properties are patched");

        assert_eq!(updated, b"# keep this\nfirst=changed\n\nthird=false");
    }

    #[test]
    fn rejects_structured_property_values() {
        let error = patch("server.properties", b"motd=MCNP", &json!({ "motd": [] }))
            .expect_err("array values cannot be represented in properties");

        assert!(error.to_string().contains("strings"));
    }
}
