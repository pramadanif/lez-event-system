pub mod emit;
pub mod error;
pub mod event;
pub mod macros;

pub use emit::{clear_events, drain_events, emit_event, event_count};
pub use error::EventError;
pub use event::{
    EventRecord, LezEvent, MAX_EVENTS_PER_TX, MAX_EVENT_PAYLOAD_BYTES, MAX_TOTAL_EVENT_BYTES,
};
