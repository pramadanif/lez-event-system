// PHASE 5 TODO: Complete decoder library
// Decoder, formatter, and CLI for LEZ events

pub mod decoder;
pub mod formatter;

pub use decoder::{decode_event, DecodedEvent, EventSchema, FieldType};
pub use formatter::{to_display, to_json};
