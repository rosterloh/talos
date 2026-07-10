use std::fmt;

use serde::{Deserialize, Serialize};

/// A ROS 2 parameter value. Mirrors the variant set of
/// `rcl_interfaces/msg/ParameterValue` without depending on any ROS 2 types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    NotSet,
    Bool(bool),
    Integer(i64),
    Double(f64),
    String(String),
    ByteArray(Vec<u8>),
    BoolArray(Vec<bool>),
    IntegerArray(Vec<i64>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
}

impl ParamValue {
    /// Human-readable type tag, e.g. for column headings.
    pub fn type_name(&self) -> &'static str {
        match self {
            ParamValue::NotSet => "not set",
            ParamValue::Bool(_) => "bool",
            ParamValue::Integer(_) => "integer",
            ParamValue::Double(_) => "double",
            ParamValue::String(_) => "string",
            ParamValue::ByteArray(_) => "byte_array",
            ParamValue::BoolArray(_) => "bool_array",
            ParamValue::IntegerArray(_) => "integer_array",
            ParamValue::DoubleArray(_) => "double_array",
            ParamValue::StringArray(_) => "string_array",
        }
    }

    /// Infer a `ParamValue` from user input (CLI argument or TUI edit field).
    ///
    /// Recognises `true`/`false`, integers, floats, `[..]` arrays, and falls
    /// back to a string (quotes are stripped). This is inference for new values;
    /// use [`parse_preserving_type`](Self::parse_preserving_type) when editing an
    /// existing parameter value.
    pub fn parse(input: &str) -> ParamValue {
        let s = input.trim();
        if let Some(inner) = s.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            return Self::parse_array(inner);
        }
        Self::parse_scalar(s)
    }

    /// Parse user input while preserving the type of an existing parameter where
    /// possible. This prevents TUI edit buffers from changing `"42"` strings
    /// into integers, and keeps display-only values such as byte arrays intact
    /// when the user submits them unchanged.
    pub fn parse_preserving_type(input: &str, current: &ParamValue) -> ParamValue {
        let s = input.trim();
        if s == current.to_string() {
            return current.clone();
        }

        match current {
            ParamValue::NotSet => Self::parse(s),
            ParamValue::Bool(_) => parse_bool(s)
                .map(ParamValue::Bool)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::Integer(_) => s
                .parse::<i64>()
                .map(ParamValue::Integer)
                .unwrap_or_else(|_| Self::parse(s)),
            ParamValue::Double(_) => s
                .parse::<f64>()
                .map(ParamValue::Double)
                .unwrap_or_else(|_| Self::parse(s)),
            ParamValue::String(_) => ParamValue::String(strip_quotes(s).to_string()),
            ParamValue::ByteArray(_) => parse_typed_array(s, |e| e.parse::<u8>().ok())
                .map(ParamValue::ByteArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::BoolArray(_) => parse_typed_array(s, parse_bool)
                .map(ParamValue::BoolArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::IntegerArray(_) => parse_typed_array(s, |e| e.parse::<i64>().ok())
                .map(ParamValue::IntegerArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::DoubleArray(_) => parse_typed_array(s, |e| e.parse::<f64>().ok())
                .map(ParamValue::DoubleArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::StringArray(_) => parse_array_elements(s)
                .map(|elems| {
                    ParamValue::StringArray(
                        elems
                            .into_iter()
                            .map(|e| strip_quotes(e).to_string())
                            .collect(),
                    )
                })
                .unwrap_or_else(|| Self::parse(s)),
        }
    }

    fn parse_scalar(s: &str) -> ParamValue {
        if let Some(b) = parse_bool(s) {
            return ParamValue::Bool(b);
        }
        if let Ok(i) = s.parse::<i64>() {
            return ParamValue::Integer(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return ParamValue::Double(f);
        }
        ParamValue::String(strip_quotes(s).to_string())
    }

    fn parse_array(inner: &str) -> ParamValue {
        let inner = inner.trim();
        if inner.is_empty() {
            return ParamValue::StringArray(Vec::new());
        }
        let elems: Vec<&str> = inner.split(',').map(str::trim).collect();
        if let Some(bools) = elems
            .iter()
            .map(|e| parse_bool(e))
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::BoolArray(bools);
        }
        if let Some(ints) = elems
            .iter()
            .map(|e| e.parse::<i64>().ok())
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::IntegerArray(ints);
        }
        if let Some(floats) = elems
            .iter()
            .map(|e| e.parse::<f64>().ok())
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::DoubleArray(floats);
        }
        ParamValue::StringArray(elems.iter().map(|e| strip_quotes(e).to_string()).collect())
    }
}

fn parse_array_elements(s: &str) -> Option<Vec<&str>> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    Some(inner.split(',').map(str::trim).collect())
}

fn parse_typed_array<T>(s: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    parse_array_elements(s)?
        .into_iter()
        .map(parse)
        .collect::<Option<Vec<_>>>()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(s)
}

/// Format an `f64` so it always reads as a float (e.g. `1.0`, never `1`),
/// keeping [`ParamValue::Double`] distinct from [`ParamValue::Integer`] when
/// displayed and re-parsed.
fn write_f64(f: &mut fmt::Formatter<'_>, v: f64) -> fmt::Result {
    if v.is_finite() && v == v.trunc() {
        write!(f, "{v:.1}")
    } else {
        write!(f, "{v}")
    }
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::NotSet => write!(f, "<not set>"),
            ParamValue::Bool(b) => write!(f, "{b}"),
            ParamValue::Integer(i) => write!(f, "{i}"),
            ParamValue::Double(d) => write_f64(f, *d),
            ParamValue::String(s) => write!(f, "{s}"),
            ParamValue::ByteArray(a) => write!(f, "[{} bytes]", a.len()),
            ParamValue::BoolArray(a) => write_seq(f, a),
            ParamValue::IntegerArray(a) => write_seq(f, a),
            ParamValue::DoubleArray(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_f64(f, *v)?;
                }
                write!(f, "]")
            }
            ParamValue::StringArray(a) => {
                write!(f, "[")?;
                for (i, s) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{s}\"")?;
                }
                write!(f, "]")
            }
        }
    }
}

fn write_seq<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    write!(f, "[")?;
    for (i, v) in items.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{v}")?;
    }
    write!(f, "]")
}

/// A named parameter with its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub value: ParamValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_value_all_variants_round_trip() {
        for value in [
            ParamValue::NotSet,
            ParamValue::Bool(true),
            ParamValue::Integer(-42),
            ParamValue::Double(3.5),
            ParamValue::String("hello".into()),
            ParamValue::ByteArray(vec![1, 2, 3]),
            ParamValue::BoolArray(vec![true, false]),
            ParamValue::IntegerArray(vec![1, 2, 3]),
            ParamValue::DoubleArray(vec![1.0, 2.5]),
            ParamValue::StringArray(vec!["a".into(), "b".into()]),
        ] {
            let info = ParamInfo {
                name: "p".into(),
                value,
            };
            let bytes = crate::protocol::codec::to_vec(&info).expect("serialize param");
            let decoded: ParamInfo = crate::protocol::codec::from_slice(&bytes).expect("deserialize param");
            assert_eq!(info, decoded);
        }
    }

    #[test]
    fn param_value_parse_infers_scalar_types() {
        assert_eq!(ParamValue::parse("true"), ParamValue::Bool(true));
        assert_eq!(ParamValue::parse("False"), ParamValue::Bool(false));
        assert_eq!(ParamValue::parse("42"), ParamValue::Integer(42));
        assert_eq!(ParamValue::parse("-7"), ParamValue::Integer(-7));
        assert_eq!(ParamValue::parse("3.14"), ParamValue::Double(3.14));
        assert_eq!(ParamValue::parse("1.0"), ParamValue::Double(1.0));
        assert_eq!(
            ParamValue::parse("hello"),
            ParamValue::String("hello".into())
        );
        // Quoted digits stay a string rather than becoming a number.
        assert_eq!(
            ParamValue::parse("\"123\""),
            ParamValue::String("123".into())
        );
    }

    #[test]
    fn param_value_parse_infers_arrays() {
        assert_eq!(
            ParamValue::parse("[1, 2, 3]"),
            ParamValue::IntegerArray(vec![1, 2, 3])
        );
        assert_eq!(
            ParamValue::parse("[1.5, 2.0]"),
            ParamValue::DoubleArray(vec![1.5, 2.0])
        );
        assert_eq!(
            ParamValue::parse("[true, false]"),
            ParamValue::BoolArray(vec![true, false])
        );
        assert_eq!(
            ParamValue::parse("[a, b]"),
            ParamValue::StringArray(vec!["a".into(), "b".into()])
        );
        assert_eq!(ParamValue::parse("[]"), ParamValue::StringArray(vec![]));
    }

    #[test]
    fn param_value_parse_preserving_type_keeps_existing_string_scalars() {
        assert_eq!(
            ParamValue::parse_preserving_type("42", &ParamValue::String("old".into())),
            ParamValue::String("42".into())
        );
        assert_eq!(
            ParamValue::parse_preserving_type("false", &ParamValue::String("old".into())),
            ParamValue::String("false".into())
        );
        assert_eq!(
            ParamValue::parse_preserving_type("[1, 2]", &ParamValue::String("old".into())),
            ParamValue::String("[1, 2]".into())
        );
    }

    #[test]
    fn param_value_parse_preserving_type_keeps_unchanged_display_only_values() {
        let bytes = ParamValue::ByteArray(vec![1, 2, 3]);
        assert_eq!(
            ParamValue::parse_preserving_type(&bytes.to_string(), &bytes),
            bytes
        );

        let strings = ParamValue::StringArray(vec!["a,b".into()]);
        assert_eq!(
            ParamValue::parse_preserving_type(&strings.to_string(), &strings),
            strings
        );
    }

    #[test]
    fn param_value_display_round_trips_through_parse() {
        for value in [
            ParamValue::Bool(true),
            ParamValue::Integer(42),
            ParamValue::Double(1.0),
            ParamValue::Double(3.14),
            ParamValue::String("hello".into()),
            ParamValue::IntegerArray(vec![1, 2, 3]),
            ParamValue::DoubleArray(vec![1.0, 2.5]),
            ParamValue::BoolArray(vec![true, false]),
        ] {
            let shown = value.to_string();
            assert_eq!(ParamValue::parse(&shown), value, "round trip via '{shown}'");
        }
    }
}
