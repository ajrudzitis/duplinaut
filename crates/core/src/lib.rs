//! Native duplicity archive parsing and reconstruction.
//!
//! Parsing and reconstruction run natively; upstream tools are test producers.
pub mod generated;
pub mod delta;
pub mod manifest_token;
pub mod path;
pub mod source;

pub mod manifest;
pub mod names;
mod verified;
