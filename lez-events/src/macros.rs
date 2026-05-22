/// Implement the [`LezEvent`](crate::LezEvent) trait for a Borsh-serialisable event struct.
///
/// # Examples
///
/// ```rust
/// use borsh::BorshSerialize;
/// use lez_events::impl_lez_event;
///
/// #[derive(BorshSerialize)]
/// pub struct TransferCompleted {
///     pub amount: u64,
/// }
/// impl_lez_event!(TransferCompleted, discriminant = 0x0002);
/// ```
///
/// Optionally override `schema_version` (default = 1):
/// ```rust
/// # use borsh::BorshSerialize;
/// # use lez_events::impl_lez_event;
/// # #[derive(BorshSerialize)] pub struct MyEvent;
/// impl_lez_event!(MyEvent, discriminant = 0x0010, schema_version = 2);
/// ```
#[macro_export]
macro_rules! impl_lez_event {
    ($type:ty, discriminant = $disc:expr) => {
        impl $crate::LezEvent for $type {
            const DISCRIMINANT: u64 = $disc;
        }
    };

    ($type:ty, discriminant = $disc:expr, schema_version = $ver:expr) => {
        impl $crate::LezEvent for $type {
            const DISCRIMINANT: u64 = $disc;
            const SCHEMA_VERSION: u8 = $ver;
        }
    };
}
