use serde_yaml::Value;

fn setting_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn get_setting_value<'a>(settings: &'a Value, dotted_key: &str) -> Option<&'a Value> {
    let mut cursor = settings;
    for segment in dotted_key.split('.') {
        match cursor {
            Value::Mapping(map) => {
                let key = Value::String(segment.to_string());
                cursor = map.get(&key)?;
            }
            _ => return None,
        }
    }
    Some(cursor)
}

pub(crate) fn get_setting_string(settings: &Value, dotted_key: &str) -> Option<String> {
    get_setting_value(settings, dotted_key).and_then(setting_value_to_string)
}
