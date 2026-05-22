//! Integration tests for EventRecord Borsh encoding determinism.

use borsh::BorshDeserialize;
use lez_events::EventRecord;

fn make_record(sequence: u32, discriminant: u64, payload: &[u8]) -> EventRecord {
    EventRecord {
        program_id: [0u8; 32],
        sequence,
        discriminant,
        schema_version: 1,
        payload: payload.to_vec(),
    }
}

#[test]
fn same_input_produces_identical_bytes() {
    let r1 = make_record(42, 0x0001, b"hello");
    let r2 = make_record(42, 0x0001, b"hello");
    assert_eq!(
        borsh::to_vec(&r1).unwrap(),
        borsh::to_vec(&r2).unwrap(),
        "Borsh encoding must be deterministic"
    );
}

#[test]
fn round_trip_encode_decode() {
    let original = make_record(7, 0xDEAD, b"test payload");
    let bytes = borsh::to_vec(&original).unwrap();
    let decoded = EventRecord::deserialize(&mut &bytes[..]).unwrap();
    assert_eq!(original, decoded, "round-trip must produce identical struct");
}

#[test]
fn different_discriminant_produces_different_bytes() {
    let r1 = make_record(0, 0x0001, b"same");
    let r2 = make_record(0, 0x0002, b"same");
    assert_ne!(
        borsh::to_vec(&r1).unwrap(),
        borsh::to_vec(&r2).unwrap(),
        "different discriminants must produce different bytes"
    );
}

#[test]
fn field_order_matches_spec() {
    // Verify wire format: program_id(32) | sequence(4) | discriminant(8) |
    //                     schema_version(1) | payload_len(4) | payload
    let record = EventRecord {
        program_id: [0xABu8; 32],
        sequence: 1u32,
        discriminant: 0x00000000_0000FFFFu64,
        schema_version: 1u8,
        payload: vec![0x42u8],
    };
    let bytes = borsh::to_vec(&record).unwrap();

    // First 32 bytes → program_id
    assert_eq!(&bytes[0..32], &[0xABu8; 32]);
    // Bytes 32..36 → sequence = 1 (LE)
    assert_eq!(&bytes[32..36], &1u32.to_le_bytes());
    // Bytes 36..44 → discriminant (LE)
    assert_eq!(&bytes[36..44], &0x00000000_0000FFFFu64.to_le_bytes());
    // Byte 44 → schema_version = 1
    assert_eq!(bytes[44], 1u8);
    // Bytes 45..49 → payload length = 1 (LE)
    assert_eq!(&bytes[45..49], &1u32.to_le_bytes());
    // Byte 49 → payload[0] = 0x42
    assert_eq!(bytes[49], 0x42u8);
}
