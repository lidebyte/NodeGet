use crate::error::{NodegetError, anyhow_to_nodeget_error};
use anyhow::Result;
use serde_json::value::RawValue;

pub fn generate_error_message(error_id: impl Into<i128>, error_message: &str) -> serde_json::Value {
    let json_error = crate::utils::JsonError {
        error_id: error_id.into(),
        error_message: error_message.to_string(),
    };
    serde_json::to_value(json_error).unwrap_or_else(|_| {
        serde_json::json!({
            "error_id": 101,
            "error_message": "Failed to serialize error"
        })
    })
}

pub fn error_to_raw(code: impl Into<i128>, msg: &str) -> Result<Box<RawValue>> {
    let json_error = crate::utils::JsonError {
        error_id: code.into(),
        error_message: msg.to_string(),
    };
    serde_json::value::to_raw_value(&json_error)
        .map_err(|e| NodegetError::SerializationError(e.to_string()).into())
}

pub fn nodeget_error_to_raw(error: &NodegetError) -> Result<Box<RawValue>> {
    let json_error = error.to_json_error();
    serde_json::value::to_raw_value(&json_error)
        .map_err(|e| NodegetError::SerializationError(e.to_string()).into())
}

pub fn anyhow_error_to_raw(error: &anyhow::Error) -> Result<Box<RawValue>> {
    let nodeget_error = anyhow_to_nodeget_error(error);
    nodeget_error_to_raw(&nodeget_error)
}
