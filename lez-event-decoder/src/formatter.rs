use crate::decoder::DecodedEvent;

/// Renders a [`DecodedEvent`] as a pretty-printed JSON string.
pub fn to_json(decoded: &DecodedEvent) -> String {
    serde_json::to_string_pretty(decoded).unwrap_or_else(|_| "{}".to_string())
}

/// Renders a [`DecodedEvent`] as a human-readable, terminal-friendly string.
///
/// Example output:
/// ```text
/// [seq=0] InsufficientFunds  [FAILED TX]  (program: 0xabcd...1234)
///   account:   0xdeadbeef...cafebabe
///   requested: 1000000
///   available: 500
/// ```
pub fn to_display(decoded: &DecodedEvent) -> String {
    let failed = if decoded.from_failed_tx {
        "  [FAILED TX]"
    } else {
        ""
    };
    let mut out = format!(
        "[seq={}] {}{}  (program: {})\n",
        decoded.sequence, decoded.event_name, failed, decoded.program_id
    );
    if decoded.fields.is_empty() {
        out.push_str(&format!("  raw: {}\n", decoded.raw_payload_hex));
    } else {
        for (name, value) in &decoded.fields {
            out.push_str(&format!("  {name}: {value}\n"));
        }
    }
    out
}
