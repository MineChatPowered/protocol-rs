//! Custom CBOR serialization with spec-compliant integer keys.
//!
//! The MineChat protocol specifies that all CBOR map keys MUST use integer indices (0, 1, 2, ...)
//! per section 6. This module provides serialization and deserialization that comply with this
//! requirement.
//!
//! Serialization uses serde_cbor's packed format which outputs struct field names as indices.
//! Deserialization accepts both integer keys and string keys.

use serde::{Deserialize, Serialize};
use serde_cbor::Value as CborValue;
use std::collections::BTreeMap;
use std::fmt;

/// Result type for CBOR operations
pub type CborResult<T> = std::result::Result<T, CborError>;

/// Error type for CBOR operations
#[derive(Debug)]
pub enum CborError {
    /// Serde serialization error
    Serde(serde_cbor::Error),
    /// Custom error message
    Custom(String),
}

impl fmt::Display for CborError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CborError::Serde(e) => write!(f, "CBOR error: {}", e),
            CborError::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for CborError {}

impl From<serde_cbor::Error> for CborError {
    fn from(e: serde_cbor::Error) -> Self {
        CborError::Serde(e)
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
pub fn serialize_envelope<P: Serialize>(packet_type: i32, payload: &P) -> CborResult<Vec<u8>> {
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
pub fn serialize<T: Serialize>(value: &T) -> CborResult<Vec<u8>> {
    let mut buf = Vec::new();
    value.serialize(&mut serde_cbor::ser::Serializer::new(&mut buf).packed_format())?;
    Ok(buf)
}

/// Deserializes a typed envelope, accepting both integer keys (spec-compliant)
/// and string keys (for robustness).
pub fn deserialize_envelope<P: for<'de> Deserialize<'de>>(data: &[u8]) -> CborResult<(i32, P)> {
    let value: CborValue = serde_cbor::de::from_slice(data)?;

    match value {
        CborValue::Map(mut map) => {
            debug_assert_eq!(
                map.len(),
                2,
                "Envelope must have exactly 2 keys (packet_type and payload)"
            );

            let packet_type_val = map
                .remove(&CborValue::Integer(0))
                .or_else(|| map.remove(&CborValue::Text("0".into())))
                .ok_or_else(|| CborError::Custom("missing packet_type".into()))?;

            let packet_type = extract_int_from_value(&packet_type_val)
                .ok_or_else(|| CborError::Custom("packet_type is not an integer".into()))?;

            let payload_value = map
                .remove(&CborValue::Integer(1))
                .or_else(|| map.remove(&CborValue::Text("1".into())))
                .ok_or_else(|| CborError::Custom("missing payload".into()))?;

            debug_assert!(map.is_empty(), "Envelope has unexpected extra keys");

            let payload_bytes = serde_cbor::to_vec(&payload_value).map_err(CborError::Serde)?;
            let payload: P = serde_cbor::de::from_slice(&payload_bytes)
                .map_err(|e| CborError::Custom(format!("payload deserialize error: {}", e)))?;

            Ok((packet_type, payload))
        }
        _ => Err(CborError::Custom("expected map envelope".into())),
    }
}

/// Deserializes from CBOR bytes, accepting both integer and string keys.
pub fn deserialize<T: for<'de> Deserialize<'de>>(data: &[u8]) -> CborResult<T> {
    let value: CborValue = serde_cbor::de::from_slice(data)?;
    let normalized = normalize_keys(&value);
    let bytes = serde_cbor::to_vec(&normalized)?;
    serde_cbor::de::from_slice(&bytes).map_err(CborError::Serde)
}

/// Normalizes CBOR Value by converting string keys to integer keys where applicable.
///
/// This function walks the CBOR value tree and converts string keys like "0", "1", "2"
/// to their integer equivalents, making the data compatible with specs that require
/// integer keys.
fn normalize_keys(value: &CborValue) -> CborValue {
    match value {
        CborValue::Map(map) => {
            let mut new_map: BTreeMap<CborValue, CborValue> = BTreeMap::new();
            for (k, v) in map {
                let new_key = match k {
                    CborValue::Text(s) => {
                        if let Ok(i) = s.parse::<i32>() {
                            CborValue::Integer(i as i128)
                        } else {
                            CborValue::Text(s.clone())
                        }
                    }
                    CborValue::Integer(i) => CborValue::Integer(*i),
                    _ => k.clone(),
                };
                new_map.insert(new_key, normalize_keys(v));
            }
            CborValue::Map(new_map)
        }
        CborValue::Array(arr) => CborValue::Array(arr.iter().map(normalize_keys).collect()),
        CborValue::Tag(tag, inner) => CborValue::Tag(*tag, Box::new(normalize_keys(inner))),
        _ => value.clone(),
    }
}

fn extract_int_from_value(v: &CborValue) -> Option<i32> {
    match v {
        CborValue::Integer(i) => Some(*i as i32),
        _ => None,
    }
}
