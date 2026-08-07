//! 实例配置的跨文档校验规则。
//!
//! 这里故意只实现能够从文件内容直接证明的规则。版本化服务端可能拥有额外
//! 配置语义，未知字段会继续交给原始配置编辑器，不会因为 Core 不认识它们
//! 就被当成错误。

use std::collections::BTreeMap;
use std::net::IpAddr;

use nexus_domain::ConfigValidationIssue;
use nexus_domain::ConfigValidationResult;
use nexus_domain::ConfigValidationSeverity;
use nexus_domain::Instance;
use nexus_domain::InstanceKind;
use serde_json::Value;

/// 从已解析的配置文档和可选 EULA 文件生成实例级诊断。
pub(crate) fn validate(
    instance: &Instance,
    documents: &[Value],
    eula: Option<&str>,
) -> ConfigValidationResult {
    let mut checked_documents = documents
        .iter()
        .filter_map(|document| document.get("path").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if requires_java_eula(instance.kind()) {
        checked_documents.push("eula.txt".to_owned());
    }
    checked_documents.sort();

    let mut issues = Vec::new();
    let mut listeners = BTreeMap::<u16, Vec<(String, String)>>::new();

    for document in documents {
        let Some(path) = document.get("path").and_then(Value::as_str) else {
            continue;
        };
        let Some(values) = document.get("values") else {
            continue;
        };

        if is_server_properties(path) {
            validate_server_properties(values, path, &mut listeners, &mut issues);
        }
        if instance.kind() == InstanceKind::Geyser && is_yaml_document(path) {
            validate_geyser_configuration(values, path, &mut listeners, &mut issues);
        }
    }

    if requires_java_eula(instance.kind()) {
        validate_eula(eula, &mut issues);
    }
    validate_listener_conflicts(listeners, &mut issues);

    ConfigValidationResult::new(checked_documents, issues)
}

fn validate_server_properties(
    values: &Value,
    path: &str,
    listeners: &mut BTreeMap<u16, Vec<(String, String)>>,
    issues: &mut Vec<ConfigValidationIssue>,
) {
    for field in ["server-port", "server-portv6", "query.port", "rcon.port"] {
        let Some(value) = object_field(values, field) else {
            continue;
        };
        let port = match parse_port(value) {
            Ok(port) => port,
            Err(PortValueError::Invalid) => {
                push_issue(
                    issues,
                    "PORT_VALUE_INVALID",
                    ConfigValidationSeverity::Error,
                    path,
                    Some(field),
                    "The configured port must be an integer",
                    None,
                    None,
                );
                continue;
            }
            Err(PortValueError::OutOfRange) => {
                push_issue(
                    issues,
                    "PORT_OUT_OF_RANGE",
                    ConfigValidationSeverity::Error,
                    path,
                    Some(field),
                    "The configured port must be between 1 and 65535",
                    None,
                    None,
                );
                continue;
            }
        };

        let enabled = match field {
            "query.port" => boolean_field(values, "enable-query", path, issues),
            "rcon.port" => boolean_field(values, "enable-rcon", path, issues),
            _ => true,
        };
        if enabled {
            listeners
                .entry(port)
                .or_default()
                .push((path.to_owned(), field.to_owned()));
        }
    }

    if let Some(value) = object_field(values, "server-ip") {
        match value {
            Value::String(address) if address.trim().is_empty() => {}
            Value::String(address) if address.parse::<IpAddr>().is_ok() => {}
            _ => push_issue(
                issues,
                "BIND_ADDRESS_INVALID",
                ConfigValidationSeverity::Error,
                path,
                Some("server-ip"),
                "The configured server-ip must be an IP literal or an empty string",
                None,
                None,
            ),
        }
    }

    for field in ["enable-query", "enable-rcon"] {
        if let Some(value) = object_field(values, field)
            && boolean_field_value(value).is_none()
        {
            push_issue(
                issues,
                "BOOLEAN_VALUE_INVALID",
                ConfigValidationSeverity::Error,
                path,
                Some(field),
                "The configured value must be true or false",
                None,
                None,
            );
        }
    }

    if boolean_field_value(object_field(values, "enable-rcon").unwrap_or(&Value::Null))
        == Some(true)
        && object_field(values, "rcon.password")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        push_issue(
            issues,
            "RCON_PASSWORD_MISSING",
            ConfigValidationSeverity::Error,
            path,
            Some("rcon.password"),
            "RCON is enabled but rcon.password is empty or missing",
            None,
            None,
        );
    }
}

fn validate_geyser_configuration(
    values: &Value,
    path: &str,
    listeners: &mut BTreeMap<u16, Vec<(String, String)>>,
    issues: &mut Vec<ConfigValidationIssue>,
) {
    if let Some(value) = nested_field(values, &["bedrock", "address"]) {
        match value {
            Value::String(address) if address.parse::<IpAddr>().is_ok() => {}
            _ => push_issue(
                issues,
                "BEDROCK_BIND_ADDRESS_INVALID",
                ConfigValidationSeverity::Error,
                path,
                Some("bedrock.address"),
                "The Bedrock bind address must be an IP literal",
                None,
                None,
            ),
        }
    }

    if let Some(value) = nested_field(values, &["bedrock", "port"]) {
        if let Some(port) = validated_port(value, path, "bedrock.port", issues) {
            listeners
                .entry(port)
                .or_default()
                .push((path.to_owned(), "bedrock.port".to_owned()));
        }
    }

    if let Some(value) = nested_field(values, &["remote", "address"])
        && !matches!(value, Value::String(address) if !address.trim().is_empty())
    {
        push_issue(
            issues,
            "REMOTE_ADDRESS_INVALID",
            ConfigValidationSeverity::Error,
            path,
            Some("remote.address"),
            "The Java backend address must not be empty",
            None,
            None,
        );
    }
    if let Some(value) = nested_field(values, &["remote", "port"]) {
        let _ = validated_port(value, path, "remote.port", issues);
    }
}

fn validate_eula(eula: Option<&str>, issues: &mut Vec<ConfigValidationIssue>) {
    let Some(eula) = eula else {
        push_issue(
            issues,
            "EULA_MISSING",
            ConfigValidationSeverity::Warning,
            "eula.txt",
            Some("eula"),
            "eula.txt is missing; the server will normally require explicit acceptance",
            None,
            None,
        );
        return;
    };

    if eula
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("eula=true"))
    {
        return;
    }
    push_issue(
        issues,
        "EULA_NOT_ACCEPTED",
        ConfigValidationSeverity::Error,
        "eula.txt",
        Some("eula"),
        "eula.txt must contain eula=true before a Java server can start",
        None,
        None,
    );
}

fn validate_listener_conflicts(
    listeners: BTreeMap<u16, Vec<(String, String)>>,
    issues: &mut Vec<ConfigValidationIssue>,
) {
    for (port, entries) in listeners {
        if entries.len() < 2 {
            continue;
        }
        let (related_path, related_field) = &entries[0];
        for (path, field) in entries.iter().skip(1) {
            if path == related_path && field == related_field {
                continue;
            }
            push_issue(
                issues,
                "LISTEN_PORT_CONFLICT",
                ConfigValidationSeverity::Warning,
                path,
                Some(field),
                &format!(
                    "Listening port {port} is also declared by another enabled service; verify bind addresses"
                ),
                Some(related_path),
                Some(related_field),
            );
        }
    }
}

fn boolean_field(
    values: &Value,
    field: &str,
    path: &str,
    issues: &mut Vec<ConfigValidationIssue>,
) -> bool {
    let Some(value) = object_field(values, field) else {
        return false;
    };
    match boolean_field_value(value) {
        Some(enabled) => enabled,
        None => {
            push_issue(
                issues,
                "BOOLEAN_VALUE_INVALID",
                ConfigValidationSeverity::Error,
                path,
                Some(field),
                "The configured value must be true or false",
                None,
                None,
            );
            false
        }
    }
}

fn validated_port(
    value: &Value,
    path: &str,
    field: &str,
    issues: &mut Vec<ConfigValidationIssue>,
) -> Option<u16> {
    match parse_port(value) {
        Ok(port) => Some(port),
        Err(PortValueError::Invalid) => {
            push_issue(
                issues,
                "PORT_VALUE_INVALID",
                ConfigValidationSeverity::Error,
                path,
                Some(field),
                "The configured port must be an integer",
                None,
                None,
            );
            None
        }
        Err(PortValueError::OutOfRange) => {
            push_issue(
                issues,
                "PORT_OUT_OF_RANGE",
                ConfigValidationSeverity::Error,
                path,
                Some(field),
                "The configured port must be between 1 and 65535",
                None,
                None,
            );
            None
        }
    }
}

fn object_field<'a>(values: &'a Value, field: &str) -> Option<&'a Value> {
    values.as_object()?.get(field)
}

fn nested_field<'a>(values: &'a Value, fields: &[&str]) -> Option<&'a Value> {
    fields
        .iter()
        .try_fold(values, |current, field| current.as_object()?.get(*field))
}

fn boolean_field_value(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::String(value) if value.eq_ignore_ascii_case("true") => Some(true),
        Value::String(value) if value.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    }
}

fn parse_port(value: &Value) -> Result<u16, PortValueError> {
    let number = match value {
        Value::Number(value) => value.as_i64().ok_or(PortValueError::Invalid)?,
        Value::String(value) => value
            .trim()
            .parse::<i64>()
            .map_err(|_| PortValueError::Invalid)?,
        _ => return Err(PortValueError::Invalid),
    };
    let port = u16::try_from(number).map_err(|_| PortValueError::OutOfRange)?;
    if port == 0 {
        return Err(PortValueError::OutOfRange);
    }
    Ok(port)
}

/// 统一创建诊断，避免各规则在可选关联位置上重复拼接领域值对象。
#[allow(clippy::too_many_arguments)]
fn push_issue(
    issues: &mut Vec<ConfigValidationIssue>,
    code: &str,
    severity: ConfigValidationSeverity,
    path: &str,
    field: Option<&str>,
    message: &str,
    related_path: Option<&str>,
    related_field: Option<&str>,
) {
    issues.push(ConfigValidationIssue::new(
        code.to_owned(),
        severity,
        path.to_owned(),
        field.map(str::to_owned),
        message.to_owned(),
        related_path.map(str::to_owned),
        related_field.map(str::to_owned),
    ));
}

fn is_server_properties(path: &str) -> bool {
    path.rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("server.properties"))
}

fn is_yaml_document(path: &str) -> bool {
    path.ends_with(".yml") || path.ends_with(".yaml")
}

fn requires_java_eula(kind: InstanceKind) -> bool {
    matches!(
        kind,
        InstanceKind::Vanilla
            | InstanceKind::Paper
            | InstanceKind::Fabric
            | InstanceKind::NeoForge
            | InstanceKind::Forge
            | InstanceKind::Bukkit
            | InstanceKind::Spigot
            | InstanceKind::Purpur
            | InstanceKind::Pufferfish
            | InstanceKind::Folia
            | InstanceKind::Leaf
            | InstanceKind::Mohist
            | InstanceKind::Magma
            | InstanceKind::Sponge
            | InstanceKind::Arclight
            | InstanceKind::Youer
            | InstanceKind::Silkard
            | InstanceKind::CatServer
    )
}

#[derive(Clone, Copy)]
enum PortValueError {
    Invalid,
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use nexus_domain::ConfigValidationSeverity;
    use nexus_domain::InstanceKind;
    use serde_json::json;

    use super::validate;

    #[test]
    fn reports_java_server_cross_file_port_conflicts_and_missing_eula() {
        let instance = test_instance(InstanceKind::Paper);
        let documents = vec![
            json!({
                "path": "server.properties",
                "values": {
                    "server-port": 25565,
                    "enable-query": true,
                    "query.port": 25565,
                    "enable-rcon": true,
                    "rcon.password": "secret",
                    "rcon.port": 25566
                }
            }),
            json!({
                "path": "nested/settings.yml",
                "values": { "enabled": true }
            }),
        ];

        let result = validate(&instance, &documents, None);

        assert!(result.valid());
        assert!(result.checked_documents().contains(&"eula.txt".to_owned()));
        assert!(result.issues().iter().any(|issue| {
            issue.code() == "EULA_MISSING" && issue.severity() == ConfigValidationSeverity::Warning
        }));
        assert!(result.issues().iter().any(|issue| {
            issue.code() == "LISTEN_PORT_CONFLICT"
                && issue.path() == "server.properties"
                && issue.related_field() == Some("server-port")
        }));
    }

    #[test]
    fn validates_geyser_bedrock_and_remote_fields() {
        let instance = test_instance(InstanceKind::Geyser);
        let documents = vec![json!({
            "path": "config.yml",
            "values": {
                "bedrock": { "address": "not-an-ip", "port": 0 },
                "remote": { "address": "", "port": 70000 }
            }
        })];

        let result = validate(&instance, &documents, None);

        assert!(!result.valid());
        assert!(result.issues().iter().any(|issue| {
            issue.code() == "BEDROCK_BIND_ADDRESS_INVALID"
                && issue.field() == Some("bedrock.address")
        }));
        assert_eq!(
            result
                .issues()
                .iter()
                .filter(|issue| issue.code() == "PORT_OUT_OF_RANGE")
                .count(),
            2
        );
    }

    fn test_instance(kind: InstanceKind) -> nexus_domain::Instance {
        use std::collections::BTreeMap;

        use nexus_domain::InstanceCreate;
        use nexus_domain::InstanceId;
        use nexus_domain::LaunchConfig;

        InstanceCreate::new(
            InstanceId::new("validation".to_owned()).expect("test instance ID is valid"),
            "Validation".to_owned(),
            kind,
            "instances/validation".to_owned(),
            LaunchConfig::new(
                "java".to_owned(),
                Vec::new(),
                BTreeMap::new(),
                "stop".to_owned(),
                30,
            ),
        )
        .expect("test instance is valid")
        .into_instance()
        .expect("test instance is created")
    }
}
