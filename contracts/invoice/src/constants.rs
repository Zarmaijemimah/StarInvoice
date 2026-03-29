/// Storage TTL (Time-To-Live) constants for Soroban persistent storage.
/// Minimum TTL threshold before extending (in ledgers).
pub const TTL_THRESHOLD: u32 = 518400; // ~30 days (assuming 5-second ledgers)

/// TTL extension duration (in ledgers).
/// When extending TTL, entries will be extended to this duration.
pub const TTL_EXTEND_TO: u32 = 1036800; // ~60 days (assuming 5-second ledgers)

/// Maximum length of invoice description in bytes.
pub const MAX_DESCRIPTION_LEN: u32 = 256;

/// Maximum allowed invoice amount to prevent excessively large values.
/// Set to 10 billion stroops (0.1 billion units of the base token).
pub const MAX_INVOICE_AMOUNT: i128 = 10_000_000_000_000;