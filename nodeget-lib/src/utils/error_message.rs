// 101: Json Parse Error
// 102: Permission Denied
// 103: Database Error
// 104: Unable to connect agent
// 105: Not Found in Database
// 106: Uuid Not Found

#[cfg(feature = "for-server")]
pub fn generate_error_message(error_id: impl Into<i128>, error_message: &str) -> serde_json::Value {
    serde_json::json!({
        "error_id": error_id.into(),
        "error_message": error_message
    })
}
