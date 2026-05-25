use lez_events::drain_events;

pub const MAGIC_BYTES: [u8; 4] = [0x4C, 0x45, 0x5A, 0x45]; // "LEZE"
pub const WIRE_VERSION: u8 = 1;

/// Wraps program execution. Automatically drains events and commits them as a framed
/// stream to the RISC0 journal before exit or panic.
pub fn execute_program<F, R>(f: F) -> R
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    // Use a drop guard to ensure events are flushed even if a panic aborts the closure
    // without unwinding (or if unwinding occurs). In RISC0 guest, panics usually abort,
    // so we also set a panic hook to catch aborts.

    // Set up a panic hook to flush events before the guest aborts.
    #[cfg(target_os = "zkvm")]
    {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            flush_events();
            original_hook(info);
        }));
    }

    let result = std::panic::catch_unwind(f);

    flush_events();

    match result {
        Ok(res) => res,
        Err(err) => std::panic::resume_unwind(err),
    }
}

fn flush_events() {
    let events = drain_events();
    if events.is_empty() {
        return;
    }

    let mut frame = Vec::new();
    frame.extend_from_slice(b"LEZE");
    frame.push(1); // Version
    frame.extend_from_slice(&(events.len() as u32).to_le_bytes()); // Count

    for event in events {
        frame.extend_from_slice(&borsh::to_vec(&event).expect("borsh encode"));
    }

    #[cfg(target_os = "zkvm")]
    {
        risc0_zkvm::guest::env::commit_slice(&frame);
    }

    #[cfg(not(target_os = "zkvm"))]
    {
        // In a non-zkVM environment (e.g. testing or demo), we just print the framed hex
        println!(
            "[HOST MOCK] Frame committed to journal: {}",
            hex::encode(&frame)
        );
    }
}
