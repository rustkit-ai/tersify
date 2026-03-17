use crate::error::Result;
use serde_json::Value;

/// Compact JSON for LLM context:
/// - Remove whitespace formatting
/// - Strip null values (carry no information)
/// - Strip empty strings and empty arrays/objects at leaf level
pub fn compress(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input)?;
    let compacted = clean(value);
    Ok(serde_json::to_string(&compacted)?)
}

fn clean(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let filtered: serde_json::Map<String, Value> = map
                .into_iter()
                .filter_map(|(k, v)| {
                    let cleaned = clean(v);
                    match &cleaned {
                        Value::Null => None,
                        Value::String(s) if s.is_empty() => None,
                        Value::Array(a) if a.is_empty() => None,
                        Value::Object(o) if o.is_empty() => None,
                        _ => Some((k, cleaned)),
                    }
                })
                .collect();
            Value::Object(filtered)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(clean).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_nulls_and_empties() {
        let input = r#"{"name":"foo","empty":"","nothing":null,"arr":[],"data":{"val":1}}"#;
        let out = compress(input).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("nothing").is_none());
        assert!(v.get("empty").is_none());
        assert!(v.get("arr").is_none());
        assert_eq!(v["name"], "foo");
        assert_eq!(v["data"]["val"], 1);
    }

    #[test]
    fn invalid_json_returns_err() {
        assert!(compress("not json").is_err());
    }
}
