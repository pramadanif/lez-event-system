use lez_events::EventRecord;
fn main() {
    let r = EventRecord {
        program_id: [0u8; 32],
        sequence: 0,
        discriminant: 1,
        schema_version: 1,
        schema_hash: [0u8; 32],
        payload: b"hello".to_vec(),
    };
    let mut journal = Vec::new();
    journal.extend_from_slice(b"LEZE");
    journal.push(1); // version
    journal.extend_from_slice(&1u32.to_le_bytes()); // count
    let record_bytes = borsh::to_vec(&r).unwrap();
    journal.extend_from_slice(&record_bytes);
    println!("{}", hex::encode(&journal));
}
