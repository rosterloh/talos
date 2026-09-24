//! Plain JSON for `--json` output. `DynValue` and `ParamValue` are converted by
//! hand because their serde derives are externally tagged (`{"F64": 1.0}`).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value};
use talos_common::protocol::types::{DynValue, ParamValue};

/// Structs become objects (fields in message order, type name dropped),
/// arrays become arrays, bytes become a base64 string, NaN/inf become null.
pub fn dynvalue(value: &DynValue) -> Value {
    match value {
        DynValue::Bool(v) => Value::from(*v),
        DynValue::I8(v) => Value::from(*v),
        DynValue::I16(v) => Value::from(*v),
        DynValue::I32(v) => Value::from(*v),
        DynValue::I64(v) => Value::from(*v),
        DynValue::U8(v) => Value::from(*v),
        DynValue::U16(v) => Value::from(*v),
        DynValue::U32(v) => Value::from(*v),
        DynValue::U64(v) => Value::from(*v),
        DynValue::F32(v) => f32_value(*v),
        DynValue::F64(v) => Value::from(*v),
        DynValue::String(v) => Value::from(v.as_str()),
        DynValue::Bytes(v) => Value::from(STANDARD.encode(v)),
        DynValue::Array(items) => Value::Array(items.iter().map(dynvalue).collect()),
        DynValue::Struct { fields, .. } => Value::Object(
            fields
                .iter()
                .map(|(name, v)| (name.clone(), dynvalue(v)))
                .collect::<Map<_, _>>(),
        ),
    }
}

/// Same conventions as [`dynvalue`]; `NotSet` becomes null.
pub fn param_value(value: &ParamValue) -> Value {
    match value {
        ParamValue::NotSet => Value::Null,
        ParamValue::Bool(v) => Value::from(*v),
        ParamValue::Integer(v) => Value::from(*v),
        ParamValue::Double(v) => Value::from(*v),
        ParamValue::String(v) => Value::from(v.as_str()),
        ParamValue::ByteArray(v) => Value::from(STANDARD.encode(v)),
        ParamValue::BoolArray(v) => Value::from(v.clone()),
        ParamValue::IntegerArray(v) => Value::from(v.clone()),
        ParamValue::DoubleArray(v) => Value::Array(v.iter().map(|d| Value::from(*d)).collect()),
        ParamValue::StringArray(v) => Value::from(v.clone()),
    }
}

/// Widen via the shortest decimal form so `0.1f32` prints as `0.1`, not
/// `0.10000000149011612`. `Value::from(f64)` maps NaN/inf to null.
fn f32_value(v: f32) -> Value {
    Value::from(v.to_string().parse::<f64>().unwrap_or(f64::NAN))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dynvalue_converts_to_plain_json() {
        let msg = DynValue::Struct {
            type_name: "test/Msg".into(),
            fields: vec![
                ("z".into(), DynValue::I32(-3)),
                ("a".into(), DynValue::F32(0.1)),
                ("nan".into(), DynValue::F64(f64::NAN)),
                ("inf".into(), DynValue::F32(f32::INFINITY)),
                ("name".into(), DynValue::String("hi".into())),
                ("data".into(), DynValue::Bytes(vec![1, 2, 3])),
                (
                    "list".into(),
                    DynValue::Array(vec![DynValue::Bool(true), DynValue::U64(u64::MAX)]),
                ),
            ],
        };
        let value = dynvalue(&msg);
        assert_eq!(
            value,
            json!({
                "z": -3,
                "a": 0.1,
                "nan": null,
                "inf": null,
                "name": "hi",
                "data": "AQID",
                "list": [true, u64::MAX],
            })
        );
        // Fields keep message order rather than being sorted.
        let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["z", "a", "nan", "inf", "name", "data", "list"]);
    }

    #[test]
    fn param_value_converts_to_plain_json() {
        assert_eq!(param_value(&ParamValue::NotSet), json!(null));
        assert_eq!(param_value(&ParamValue::Double(2.5)), json!(2.5));
        assert_eq!(
            param_value(&ParamValue::ByteArray(vec![255])),
            json!("/w==")
        );
        assert_eq!(
            param_value(&ParamValue::DoubleArray(vec![1.0, f64::NAN])),
            json!([1.0, null])
        );
        assert_eq!(
            param_value(&ParamValue::StringArray(vec!["a".into()])),
            json!(["a"])
        );
    }
}
