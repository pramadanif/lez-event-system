use lez_events::EventRecord;
use serde::Serialize;

/// A fully decoded event with human-readable fields.
#[derive(Debug, Serialize)]
pub struct DecodedEvent {
    /// `"0x"` + lower-hex of the 32-byte program id.
    pub program_id: String,
    /// Schema-based name, or `"Unknown(0x{discriminant:04x})"` when no schema matches.
    pub event_name: String,
    pub sequence: u32,
    pub schema_version: u8,
    pub discriminant: u64,
    /// Decoded field name → string value pairs (empty for unknown events).
    pub fields: Vec<(String, String)>,
    /// Always-present fallback: lower-hex of the raw payload bytes.
    pub raw_payload_hex: String,
    /// `true` when the event came from a transaction that ultimately failed.
    pub from_failed_tx: bool,
}

/// Description of one event variant so the decoder can name fields.
pub struct EventSchema {
    pub discriminant: u64,
    pub name: String,
    pub fields: Vec<(String, FieldType)>,
}

/// Borsh-compatible primitive types that can appear in event payloads.
#[derive(Debug)]
pub enum FieldType {
    U8,
    U32,
    U64,
    U128,
    Bool,
    /// 32-byte array rendered as `"0x{hex}"`.
    Bytes32,
    /// Variable-length byte slice rendered as `"0x{hex}"`.
    VecU8,
    String,
}

/// Decode a raw [`EventRecord`] using the provided schema slice.
///
/// If no schema matches the event's discriminant, every field in the returned
/// [`DecodedEvent`] is empty and the name is `"Unknown(0x{discriminant:04x})"`.
/// This function never panics.
pub fn decode_event(record: &EventRecord, schemas: &[EventSchema]) -> DecodedEvent {
    let program_id = format!("0x{}", hex::encode(record.program_id));
    let raw_payload_hex = hex::encode(&record.payload);

    let schema = schemas
        .iter()
        .find(|s| s.discriminant == record.discriminant);

    let (event_name, fields) = match schema {
        None => (
            format!("Unknown(0x{:04x})", record.discriminant),
            Vec::new(),
        ),
        Some(s) => {
            let decoded_fields = decode_fields(&record.payload, &s.fields);
            (s.name.clone(), decoded_fields)
        }
    };

    DecodedEvent {
        program_id,
        event_name,
        sequence: record.sequence,
        schema_version: record.schema_version,
        discriminant: record.discriminant,
        fields,
        raw_payload_hex,
        from_failed_tx: false,
    }
}

/// Deserialise Borsh bytes field-by-field according to `schema_fields`.
///
/// Any parse error simply stops processing further fields and returns what was
/// decoded so far — we never panic or return an error to callers.
fn decode_fields(payload: &[u8], schema_fields: &[(String, FieldType)]) -> Vec<(String, String)> {
    let mut cursor = std::io::Cursor::new(payload);
    let mut out = Vec::new();

    for (name, field_type) in schema_fields {
        let value = read_field(&mut cursor, field_type);
        match value {
            Some(v) => out.push((name.clone(), v)),
            None => break,
        }
    }
    out
}

fn read_field(cursor: &mut std::io::Cursor<&[u8]>, ft: &FieldType) -> Option<String> {
    use std::io::Read;

    match ft {
        FieldType::U8 => {
            let mut b = [0u8; 1];
            cursor.read_exact(&mut b).ok()?;
            Some(b[0].to_string())
        }
        FieldType::Bool => {
            let mut b = [0u8; 1];
            cursor.read_exact(&mut b).ok()?;
            Some(if b[0] != 0 { "true" } else { "false" }.to_string())
        }
        FieldType::U32 => {
            let mut b = [0u8; 4];
            cursor.read_exact(&mut b).ok()?;
            Some(u32::from_le_bytes(b).to_string())
        }
        FieldType::U64 => {
            let mut b = [0u8; 8];
            cursor.read_exact(&mut b).ok()?;
            Some(u64::from_le_bytes(b).to_string())
        }
        FieldType::U128 => {
            let mut b = [0u8; 16];
            cursor.read_exact(&mut b).ok()?;
            Some(u128::from_le_bytes(b).to_string())
        }
        FieldType::Bytes32 => {
            let mut b = [0u8; 32];
            cursor.read_exact(&mut b).ok()?;
            Some(format!("0x{}", hex::encode(b)))
        }
        FieldType::VecU8 => {
            // Borsh: 4-byte LE length then bytes
            let mut len_b = [0u8; 4];
            cursor.read_exact(&mut len_b).ok()?;
            let len = u32::from_le_bytes(len_b) as usize;
            let mut data = vec![0u8; len];
            cursor.read_exact(&mut data).ok()?;
            Some(format!("0x{}", hex::encode(data)))
        }
        FieldType::String => {
            // Borsh: 4-byte LE length then UTF-8 bytes
            let mut len_b = [0u8; 4];
            cursor.read_exact(&mut len_b).ok()?;
            let len = u32::from_le_bytes(len_b) as usize;
            let mut data = vec![0u8; len];
            cursor.read_exact(&mut data).ok()?;
            Some(String::from_utf8_lossy(&data).into_owned())
        }
    }
}
