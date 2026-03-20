//! Custom CBOR serialization with spec-compliant integer keys.
//!
//! The MineChat protocol specifies that all CBOR map keys MUST use integer
//! indices (0, 1, 2...) per Section 6. This module provides serialization
//! and deserialization that comply with this requirement.
//!
//! Serialization uses serde_cbor's packed format which outputs struct field
//! names as indices. Deserialization accepts both integer keys (spec-compliant)
//! and string keys (for robustness).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Result type for CBOR operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for CBOR operations
#[derive(Debug)]
pub enum Error {
    /// Serde serialization error
    Serde(serde_cbor::Error),
    /// Custom error message
    Custom(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Serde(e) => write!(f, "CBOR error: {}", e),
            Error::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_cbor::Error> for Error {
    fn from(e: serde_cbor::Error) -> Self {
        Error::Serde(e)
    }
}

/// Envelope structure for serialization/deserialization.
///
/// This is used internally to serialize/deserialize the packet envelope
/// with its typed payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypedEnvelope<P> {
    #[serde(rename = "0")]
    packet_type: i32,
    #[serde(rename = "1")]
    payload: P,
}

/// Serializes a typed envelope using packed CBOR format.
///
/// This uses serde_cbor's packed format which outputs struct field names
/// as integer indices (0, 1, 2...) instead of strings.
pub fn serialize_envelope<P: Serialize>(packet_type: i32, payload: &P) -> Result<Vec<u8>> {
    let envelope = TypedEnvelope {
        packet_type,
        payload,
    };

    // Use packed format to get integer indices instead of string keys
    let mut buf = Vec::new();
    envelope.serialize(&mut serde_cbor::ser::Serializer::new(&mut buf).packed_format())?;
    Ok(buf)
}

/// Serializes any serializable value using packed CBOR format.
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    value.serialize(&mut serde_cbor::ser::Serializer::new(&mut buf).packed_format())?;
    Ok(buf)
}

/// Deserializes a typed envelope, accepting both integer keys (spec-compliant)
/// and string keys (for robustness).
pub fn deserialize_envelope<P: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<(i32, P)> {
    let value: serde_cbor::Value = serde_cbor::de::from_slice(data)?;

    match value {
        serde_cbor::Value::Map(mut map) => {
            debug_assert_eq!(
                map.len(),
                2,
                "Envelope must have exactly 2 keys (packet_type and payload)"
            );

            let packet_type_val = map
                .remove(&serde_cbor::Value::Integer(0))
                .or_else(|| map.remove(&serde_cbor::Value::Text("0".into())))
                .ok_or_else(|| Error::Custom("missing packet_type".into()))?;

            let packet_type = extract_int_from_value(&packet_type_val)
                .ok_or_else(|| Error::Custom("packet_type is not an integer".into()))?;

            let payload_value = map
                .remove(&serde_cbor::Value::Integer(1))
                .or_else(|| map.remove(&serde_cbor::Value::Text("1".into())))
                .ok_or_else(|| Error::Custom("missing payload".into()))?;

            debug_assert!(map.is_empty(), "Envelope has unexpected extra keys");

            let payload_bytes = serde_cbor::to_vec(&payload_value).map_err(Error::Serde)?;
            let payload: P = serde_cbor::de::from_slice(&payload_bytes)
                .map_err(|e| Error::Custom(format!("payload deserialize error: {}", e)))?;

            Ok((packet_type, payload))
        }
        _ => Err(Error::Custom("expected map envelope".into())),
    }
}

/// Deserializes from CBOR bytes, accepting both integer and string keys.
pub fn deserialize<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T> {
    let value: serde_cbor::Value = serde_cbor::de::from_slice(data)?;
    let normalized = normalize_keys(&value);
    let bytes = serde_cbor::to_vec(&normalized)?;
    serde_cbor::de::from_slice(&bytes).map_err(Error::Serde)
}

/// Normalizes CBOR Value by converting string keys to integer keys where applicable.
///
/// This function walks the CBOR value tree and converts string keys like "0", "1", "2"
/// to their integer equivalents, making the data compatible with specs that require
/// integer keys.
fn normalize_keys(value: &serde_cbor::Value) -> serde_cbor::Value {
    match value {
        serde_cbor::Value::Map(map) => {
            let mut new_map: BTreeMap<serde_cbor::Value, serde_cbor::Value> = BTreeMap::new();
            for (k, v) in map {
                let new_key = match k {
                    serde_cbor::Value::Text(s) => {
                        if let Ok(i) = s.parse::<i32>() {
                            serde_cbor::Value::Integer(i as i128)
                        } else {
                            serde_cbor::Value::Text(s.clone())
                        }
                    }
                    serde_cbor::Value::Integer(i) => serde_cbor::Value::Integer(*i),
                    _ => k.clone(),
                };
                new_map.insert(new_key, normalize_keys(v));
            }
            serde_cbor::Value::Map(new_map)
        }
        serde_cbor::Value::Array(arr) => {
            serde_cbor::Value::Array(arr.iter().map(normalize_keys).collect())
        }
        serde_cbor::Value::Tag(tag, inner) => {
            serde_cbor::Value::Tag(*tag, Box::new(normalize_keys(inner)))
        }
        _ => value.clone(),
    }
}

fn extract_int_from_value(v: &serde_cbor::Value) -> Option<i32> {
    match v {
        serde_cbor::Value::Integer(i) => Some(*i as i32),
        _ => None,
    }
}
