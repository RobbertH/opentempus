//! iCalendar (RFC 5545) support: parsing inbound feeds into normalized
//! events, expanding recurrence rules into concrete occurrences and rendering
//! outbound feeds.

pub mod expand;
pub mod generate;
pub mod parse;
pub mod tz;

pub use expand::expand;
pub use parse::ParsedEvent;
