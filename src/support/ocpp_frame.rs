//! OCPP-J message framing
//!
//! Implements the OCPP-J (JSON over WebSocket) transport protocol framing.
//! This framing is **identical** across all OCPP versions (1.6, 2.0.1, 2.1):
//!
//! - **Call**       `[2, "<uniqueId>", "<action>", {<payload>}]`
//! - **CallResult** `[3, "<uniqueId>", {<payload>}]`
//! - **CallError**  `[4, "<uniqueId>", "<errorCode>", "<errorDescription>", {<errorDetails>}]`

use serde_json::Value;
use std::fmt;

// ── Message-type constants ─────────────────────────────────────

const MSG_TYPE_CALL: u64 = 2;
const MSG_TYPE_CALL_RESULT: u64 = 3;
const MSG_TYPE_CALL_ERROR: u64 = 4;

// ── OcppFrame ──────────────────────────────────────────────────

/// A parsed OCPP-J frame (version-agnostic transport envelope).
#[derive(Debug, Clone)]
pub enum OcppFrame {
    /// `[2, uniqueId, action, payload]`
    Call {
        unique_id: String,
        action: String,
        payload: Value,
    },
    /// `[3, uniqueId, payload]`
    CallResult {
        unique_id: String,
        payload: Value,
    },
    /// `[4, uniqueId, errorCode, errorDescription, errorDetails]`
    CallError {
        unique_id: String,
        error_code: String,
        error_description: String,
        error_details: Value,
    },
}

impl OcppFrame {
    // ── Parsing ────────────────────────────────────────────

    /// Parse a raw JSON text into an `OcppFrame`.
    pub fn parse(text: &str) -> Result<Self, OcppFrameError> {
        let arr: Vec<Value> =
            serde_json::from_str(text).map_err(|e| OcppFrameError::InvalidJson(e.to_string()))?;

        if arr.is_empty() {
            return Err(OcppFrameError::EmptyArray);
        }

        let msg_type = arr[0]
            .as_u64()
            .ok_or(OcppFrameError::InvalidMessageType)?;

        match msg_type {
            MSG_TYPE_CALL => Self::parse_call(&arr),
            MSG_TYPE_CALL_RESULT => Self::parse_call_result(&arr),
            MSG_TYPE_CALL_ERROR => Self::parse_call_error(&arr),
            _ => Err(OcppFrameError::UnknownMessageType(msg_type)),
        }
    }

    fn parse_call(arr: &[Value]) -> Result<Self, OcppFrameError> {
        if arr.len() < 4 {
            return Err(OcppFrameError::MissingFields {
                expected: 4,
                got: arr.len(),
            });
        }

        let unique_id = arr[1]
            .as_str()
            .ok_or(OcppFrameError::FieldTypeMismatch("uniqueId must be a string"))?
            .to_string();
        let action = arr[2]
            .as_str()
            .ok_or(OcppFrameError::FieldTypeMismatch("action must be a string"))?
            .to_string();
        let payload = arr[3].clone();

        Ok(Self::Call {
            unique_id,
            action,
            payload,
        })
    }

    fn parse_call_result(arr: &[Value]) -> Result<Self, OcppFrameError> {
        if arr.len() < 3 {
            return Err(OcppFrameError::MissingFields {
                expected: 3,
                got: arr.len(),
            });
        }

        let unique_id = arr[1]
            .as_str()
            .ok_or(OcppFrameError::FieldTypeMismatch("uniqueId must be a string"))?
            .to_string();
        let payload = arr.get(2).cloned().unwrap_or(Value::Object(Default::default()));

        Ok(Self::CallResult { unique_id, payload })
    }

    fn parse_call_error(arr: &[Value]) -> Result<Self, OcppFrameError> {
        if arr.len() < 4 {
            return Err(OcppFrameError::MissingFields {
                expected: 4,
                got: arr.len(),
            });
        }

        let unique_id = arr[1]
            .as_str()
            .ok_or(OcppFrameError::FieldTypeMismatch("uniqueId must be a string"))?
            .to_string();
        let error_code = arr[2]
            .as_str()
            .unwrap_or("InternalError")
            .to_string();
        let error_description = arr
            .get(3)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let error_details = arr
            .get(4)
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        Ok(Self::CallError {
            unique_id,
            error_code,
            error_description,
            error_details,
        })
    }

    // ── Serialization ──────────────────────────────────────

    /// Serialize this frame to a JSON string.
    pub fn serialize(&self) -> String {
        let arr: Value = match self {
            Self::Call {
                unique_id,
                action,
                payload,
            } => Value::Array(vec![
                Value::Number(MSG_TYPE_CALL.into()),
                Value::String(unique_id.clone()),
                Value::String(action.clone()),
                payload.clone(),
            ]),

            Self::CallResult { unique_id, payload } => Value::Array(vec![
                Value::Number(MSG_TYPE_CALL_RESULT.into()),
                Value::String(unique_id.clone()),
                payload.clone(),
            ]),

            Self::CallError {
                unique_id,
                error_code,
                error_description,
                error_details,
            } => Value::Array(vec![
                Value::Number(MSG_TYPE_CALL_ERROR.into()),
                Value::String(unique_id.clone()),
                Value::String(error_code.clone()),
                Value::String(error_description.clone()),
                error_details.clone(),
            ]),
        };

        // serde_json::to_string on a Value never fails
        serde_json::to_string(&arr).unwrap()
    }

    // ── Helpers ────────────────────────────────────────────

    /// Get the unique message ID.
    pub fn unique_id(&self) -> &str {
        match self {
            Self::Call { unique_id, .. }
            | Self::CallResult { unique_id, .. }
            | Self::CallError { unique_id, .. } => unique_id,
        }
    }

    /// Create a `CallError` response for a given unique ID.
    pub fn error_response(
        unique_id: impl Into<String>,
        error_code: impl Into<String>,
        error_description: impl Into<String>,
    ) -> Self {
        Self::CallError {
            unique_id: unique_id.into(),
            error_code: error_code.into(),
            error_description: error_description.into(),
            error_details: Value::Object(Default::default()),
        }
    }

    /// Returns `true` if this is a `Call` frame.
    pub fn is_call(&self) -> bool {
        matches!(self, Self::Call { .. })
    }

    /// Returns `true` if this is a `CallResult` frame.
    pub fn is_call_result(&self) -> bool {
        matches!(self, Self::CallResult { .. })
    }

    /// Returns `true` if this is a `CallError` frame.
    pub fn is_call_error(&self) -> bool {
        matches!(self, Self::CallError { .. })
    }
}

// ── Errors ─────────────────────────────────────────────────────

/// Errors that can occur when parsing an OCPP-J frame.
#[derive(Debug)]
pub enum OcppFrameError {
    InvalidJson(String),
    EmptyArray,
    InvalidMessageType,
    UnknownMessageType(u64),
    MissingFields { expected: usize, got: usize },
    FieldTypeMismatch(&'static str),
}

impl fmt::Display for OcppFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "Invalid JSON: {}", msg),
            Self::EmptyArray => write!(f, "Empty OCPP message array"),
            Self::InvalidMessageType => write!(f, "Message type is not a number"),
            Self::UnknownMessageType(t) => write!(f, "Unknown message type: {}", t),
            Self::MissingFields { expected, got } => {
                write!(f, "Expected at least {} fields, got {}", expected, got)
            }
            Self::FieldTypeMismatch(msg) => write!(f, "Field type mismatch: {}", msg),
        }
    }
}

impl std::error::Error for OcppFrameError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_call() {
        let text = r#"[2,"abc123","BootNotification",{"chargePointVendor":"Vendor","chargePointModel":"Model"}]"#;
        let frame = OcppFrame::parse(text).unwrap();
        match frame {
            OcppFrame::Call {
                unique_id,
                action,
                payload,
            } => {
                assert_eq!(unique_id, "abc123");
                assert_eq!(action, "BootNotification");
                assert_eq!(payload["chargePointVendor"], "Vendor");
            }
            _ => panic!("Expected Call frame"),
        }
    }

    #[test]
    fn parse_call_result() {
        let text = r#"[3,"abc123",{"status":"Accepted","currentTime":"2024-01-01T00:00:00Z","interval":300}]"#;
        let frame = OcppFrame::parse(text).unwrap();
        match frame {
            OcppFrame::CallResult { unique_id, payload } => {
                assert_eq!(unique_id, "abc123");
                assert_eq!(payload["status"], "Accepted");
            }
            _ => panic!("Expected CallResult frame"),
        }
    }

    #[test]
    fn parse_call_error() {
        let text =
            r#"[4,"abc123","NotImplemented","Action not supported",{}]"#;
        let frame = OcppFrame::parse(text).unwrap();
        match frame {
            OcppFrame::CallError {
                unique_id,
                error_code,
                error_description,
                ..
            } => {
                assert_eq!(unique_id, "abc123");
                assert_eq!(error_code, "NotImplemented");
                assert_eq!(error_description, "Action not supported");
            }
            _ => panic!("Expected CallError frame"),
        }
    }

    #[test]
    fn roundtrip_call() {
        let frame = OcppFrame::Call {
            unique_id: "id1".into(),
            action: "Heartbeat".into(),
            payload: serde_json::json!({}),
        };
        let json = frame.serialize();
        let parsed = OcppFrame::parse(&json).unwrap();
        assert!(parsed.is_call());
        assert_eq!(parsed.unique_id(), "id1");
    }

    #[test]
    fn roundtrip_call_result() {
        let frame = OcppFrame::CallResult {
            unique_id: "id2".into(),
            payload: serde_json::json!({"currentTime": "2024-01-01T00:00:00Z"}),
        };
        let json = frame.serialize();
        let parsed = OcppFrame::parse(&json).unwrap();
        assert!(parsed.is_call_result());
        assert_eq!(parsed.unique_id(), "id2");
    }

    #[test]
    fn roundtrip_call_error() {
        let frame = OcppFrame::error_response("id3", "GenericError", "Something went wrong");
        let json = frame.serialize();
        let parsed = OcppFrame::parse(&json).unwrap();
        assert!(parsed.is_call_error());
        assert_eq!(parsed.unique_id(), "id3");
    }
}
