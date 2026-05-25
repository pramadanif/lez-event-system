use borsh::BorshDeserialize;
use lez_events::EventRecord;
use thiserror::Error;

use crate::runtime::{MAGIC_BYTES, WIRE_VERSION};

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Malformed magic bytes or incomplete frame header")]
    MalformedHeader,
    #[error("Unsupported wire version: {0}")]
    UnsupportedVersion(u8),
    #[error("Failed to deserialize event payload: {0}")]
    DeserializationError(String),
}

/// Parses a framed event stream from the LEZ execution journal.
///
/// Returns `(events, remaining_bytes)`. If no events frame is found at the start
/// of the journal, it returns `Ok((vec![], journal))`.
///
/// This allows minimal-invasive integration into the existing LEZ runtime flow:
/// `let (events, rest) = parse_journal(&journal_bytes)?;`
/// `let output = risc0_zkvm::serde::from_slice(rest)?;`
pub fn parse_journal(mut journal: &[u8]) -> Result<(Vec<EventRecord>, &[u8]), ParseError> {
    let mut all_events = Vec::new();

    // Loop to support multiple concatenated frames (though we currently emit one).
    while journal.len() >= 4 && journal[0..4] == MAGIC_BYTES {
        if journal.len() < 9 {
            return Err(ParseError::MalformedHeader);
        }

        let version = journal[4];
        if version != WIRE_VERSION {
            return Err(ParseError::UnsupportedVersion(version));
        }

        let count_bytes: [u8; 4] = journal[5..9].try_into().unwrap();
        let count = u32::from_le_bytes(count_bytes);

        journal = &journal[9..];

        for _ in 0..count {
            match EventRecord::deserialize(&mut journal) {
                Ok(event) => all_events.push(event),
                Err(e) => return Err(ParseError::DeserializationError(e.to_string())),
            }
        }
    }

    Ok((all_events, journal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_journal() {
        let journal = vec![0, 1, 2, 3];
        let (events, rest) = parse_journal(&journal).unwrap();
        assert!(events.is_empty());
        assert_eq!(rest, &[0, 1, 2, 3]);
    }

    #[test]
    fn test_parse_valid_frame() {
        let mut frame = MAGIC_BYTES.to_vec();
        frame.push(WIRE_VERSION);
        frame.extend_from_slice(&1u32.to_le_bytes());

        let event = EventRecord {
            program_id: [1; 32],
            sequence: 0,
            discriminant: 42,
            schema_version: 1,
            schema_hash: [2; 32],
            payload: vec![10, 20, 30],
        };
        frame.extend_from_slice(&borsh::to_vec(&event).unwrap());

        // append some mock ProgramOutput bytes
        frame.extend_from_slice(&[99, 99, 99]);

        let (events, rest) = parse_journal(&frame).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
        assert_eq!(rest, &[99, 99, 99]);
    }

    #[test]
    fn test_unsupported_version() {
        let mut frame = MAGIC_BYTES.to_vec();
        frame.push(99); // future version
        frame.extend_from_slice(&1u32.to_le_bytes());

        assert_eq!(
            parse_journal(&frame),
            Err(ParseError::UnsupportedVersion(99))
        );
    }
}
