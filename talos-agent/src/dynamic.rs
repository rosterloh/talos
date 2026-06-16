//! Runtime dynamic-message subscription (Route 1 spike).
//!
//! The static registry in [`crate::conversions`] only handles a fixed set of
//! message types; any topic whose type is not compiled in is skipped. This
//! module adds a fallback that subscribes to *any* topic type at runtime using
//! `rclrs`'s `DynamicMessage` support, which loads the message's
//! `rosidl_typesupport_introspection` library via `dlopen` and walks the
//! message structure field-by-field.
//!
//! The walker converts a [`rclrs::DynamicMessage`] into the same
//! [`DynValue`] tree the static converters produce, so the wire protocol and
//! every downstream client (CLI, TUI) are unchanged.
//!
//! Status: spike. Single nested messages, numeric/bool/string scalars, fixed
//! arrays, unbounded sequences, and fixed arrays of nested messages are
//! converted with full fidelity. Less common shapes (message sequences,
//! bounded sequences, wide/long-double/wstring scalars) currently fall back to
//! a debug-string representation and are marked with `TODO` below.

use std::error::Error;

use rclrs::{
    ArrayValue, DynamicMessage, DynamicMessageView, MessageInfo, MessageTypeName, SequenceValue,
    SimpleValue, Value,
};
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{DynValue, Timestamp};
use tokio::sync::mpsc;
use tracing::warn;

type TopicSender = mpsc::UnboundedSender<Response>;
type SubscribeResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Subscribe to `topic` of the runtime-resolved `type_name`, converting each
/// message to a [`DynValue`] tree. Mirrors the signature of the static
/// registry's `subscribe` functions so the bridge can use it as a drop-in
/// fallback.
pub fn subscribe_dynamic(
    node: &rclrs::Node,
    opts: rclrs::PrimitiveOptions<'_>,
    topic: String,
    type_name: String,
    tx: TopicSender,
) -> SubscribeResult {
    let topic_type = MessageTypeName::try_from(type_name.as_str())
        .map_err(|e| format!("invalid message type '{type_name}': {e:?}"))?;

    node.create_dynamic_subscription(
        topic_type,
        opts,
        move |msg: DynamicMessage, _info: MessageInfo| {
            let view = msg.view();
            let stamp = extract_stamp(&view).unwrap_or(Timestamp { sec: 0, nanosec: 0 });
            let data = message_to_dynvalue(&view);
            let _ = tx.send(Response::TopicData {
                topic: topic.clone(),
                type_name: type_name.clone(),
                stamp,
                data,
            });
        },
    )?;
    Ok(())
}

/// Convert a whole message view into a `DynValue::Struct`.
fn message_to_dynvalue(view: &DynamicMessageView<'_>) -> DynValue {
    let structure = view.structure();
    let mut fields = Vec::with_capacity(structure.fields.len());
    for field in &structure.fields {
        let value = match view.get(&field.name) {
            Some(v) => value_to_dynvalue(v),
            // Should not happen: the field name came from the structure itself.
            None => DynValue::String(String::new()),
        };
        fields.push((field.name.clone(), value));
    }
    DynValue::Struct {
        type_name: structure.type_name.clone(),
        fields,
    }
}

/// Convert a single field value (simple / array / sequence / bounded sequence).
fn value_to_dynvalue(value: Value<'_>) -> DynValue {
    match value {
        Value::Simple(s) => simple_to_dynvalue(s),
        Value::Array(a) => array_to_dynvalue(a),
        Value::Sequence(s) => sequence_to_dynvalue(s),
        // TODO(spike): bounded sequences. Deref to a slice like Sequence, but
        // left as a faithful debug rendering until covered + tested.
        Value::BoundedSequence(b) => DynValue::String(format!("{b:?}")),
    }
}

/// Scalar field conversion, including recursion into nested messages.
fn simple_to_dynvalue(value: SimpleValue<'_>) -> DynValue {
    match value {
        SimpleValue::Float(v) => DynValue::F32(*v),
        SimpleValue::Double(v) => DynValue::F64(*v),
        SimpleValue::Boolean(v) => DynValue::Bool(*v),
        // All three are &u8.
        SimpleValue::Char(v) | SimpleValue::Octet(v) | SimpleValue::Uint8(v) => DynValue::U8(*v),
        SimpleValue::Int8(v) => DynValue::I8(*v),
        SimpleValue::Uint16(v) => DynValue::U16(*v),
        SimpleValue::Int16(v) => DynValue::I16(*v),
        SimpleValue::Uint32(v) => DynValue::U32(*v),
        SimpleValue::Int32(v) => DynValue::I32(*v),
        SimpleValue::Uint64(v) => DynValue::U64(*v),
        SimpleValue::Int64(v) => DynValue::I64(*v),
        SimpleValue::String(v) => DynValue::String(v.to_string()),
        SimpleValue::Message(view) => message_to_dynvalue(&view),
        // TODO(spike): LongDouble, WChar, WString, BoundedString, BoundedWString.
        // Platform-specific / rarely used in robot telemetry; rendered as debug
        // text for now so no field is silently dropped.
        other => DynValue::String(format!("{other:?}")),
    }
}

/// Fixed-length array conversion.
fn array_to_dynvalue(value: ArrayValue<'_>) -> DynValue {
    match value {
        ArrayValue::DoubleArray(s) => DynValue::Array(s.iter().map(|v| DynValue::F64(*v)).collect()),
        ArrayValue::FloatArray(s) => DynValue::Array(s.iter().map(|v| DynValue::F32(*v)).collect()),
        ArrayValue::BooleanArray(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::Bool(*v)).collect())
        }
        // Byte-like fixed arrays map to Bytes for compactness.
        ArrayValue::Uint8Array(s) | ArrayValue::OctetArray(s) | ArrayValue::CharArray(s) => {
            DynValue::Bytes(s.to_vec())
        }
        ArrayValue::Int8Array(s) => DynValue::Array(s.iter().map(|v| DynValue::I8(*v)).collect()),
        ArrayValue::Uint16Array(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U16(*v)).collect())
        }
        ArrayValue::Int16Array(s) => DynValue::Array(s.iter().map(|v| DynValue::I16(*v)).collect()),
        ArrayValue::Uint32Array(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U32(*v)).collect())
        }
        ArrayValue::Int32Array(s) => DynValue::Array(s.iter().map(|v| DynValue::I32(*v)).collect()),
        ArrayValue::Uint64Array(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U64(*v)).collect())
        }
        ArrayValue::Int64Array(s) => DynValue::Array(s.iter().map(|v| DynValue::I64(*v)).collect()),
        ArrayValue::StringArray(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::String(v.to_string())).collect())
        }
        ArrayValue::MessageArray(views) => {
            DynValue::Array(views.iter().map(message_to_dynvalue).collect())
        }
        // TODO(spike): wide/bounded string arrays + long double arrays.
        other => DynValue::String(format!("{other:?}")),
    }
}

/// Unbounded sequence conversion. `Sequence<T>` dereferences to `[T]`.
fn sequence_to_dynvalue(value: SequenceValue<'_>) -> DynValue {
    match value {
        SequenceValue::DoubleSequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::F64(*v)).collect())
        }
        SequenceValue::FloatSequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::F32(*v)).collect())
        }
        SequenceValue::BooleanSequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::Bool(*v)).collect())
        }
        SequenceValue::Uint8Sequence(s)
        | SequenceValue::OctetSequence(s)
        | SequenceValue::CharSequence(s) => DynValue::Bytes(s.to_vec()),
        SequenceValue::Int8Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::I8(*v)).collect())
        }
        SequenceValue::Uint16Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U16(*v)).collect())
        }
        SequenceValue::Int16Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::I16(*v)).collect())
        }
        SequenceValue::Uint32Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U32(*v)).collect())
        }
        SequenceValue::Int32Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::I32(*v)).collect())
        }
        SequenceValue::Uint64Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::U64(*v)).collect())
        }
        SequenceValue::Int64Sequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::I64(*v)).collect())
        }
        SequenceValue::StringSequence(s) => {
            DynValue::Array(s.iter().map(|v| DynValue::String(v.to_string())).collect())
        }
        // TODO(spike): MessageSequence (e.g. PoseArray.poses) needs the
        // DynamicSequence<DynamicMessageView> iteration API wired in + tested.
        other => DynValue::String(format!("{other:?}")),
    }
}

/// Best-effort timestamp extraction so time-series clients keep working without
/// per-type knowledge: prefer `header.stamp`, then a top-level `stamp`.
fn extract_stamp(view: &DynamicMessageView<'_>) -> Option<Timestamp> {
    if let Some(Value::Simple(SimpleValue::Message(header))) = view.get("header") {
        if let Some(ts) = stamp_from_time(&header, "stamp") {
            return Some(ts);
        }
    }
    stamp_from_time(view, "stamp")
}

/// Read a `builtin_interfaces/Time` (`sec: int32`, `nanosec: uint32`) named
/// `field` out of `view`.
fn stamp_from_time(view: &DynamicMessageView<'_>, field: &str) -> Option<Timestamp> {
    let Some(Value::Simple(SimpleValue::Message(time))) = view.get(field) else {
        return None;
    };
    let sec = match time.get("sec") {
        Some(Value::Simple(SimpleValue::Int32(v))) => *v,
        _ => return None,
    };
    let nanosec = match time.get("nanosec") {
        Some(Value::Simple(SimpleValue::Uint32(v))) => *v,
        _ => return None,
    };
    Some(Timestamp { sec, nanosec })
}

/// Log helper used by the bridge when falling back to the dynamic path.
pub fn log_dynamic_fallback(topic: &str, type_name: &str) {
    warn!(
        topic = %topic,
        msg_type = %type_name,
        "no static converter; using runtime dynamic-message subscription"
    );
}
