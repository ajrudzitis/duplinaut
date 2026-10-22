//! Native duplicity archive parsing and reconstruction.
//!
//! Parsing and reconstruction run natively; upstream tools are test producers.
//!
//! Low-level OpenPGP decoding is private to the composed reader; external callers
//! cannot construct decoded/authentication assertions to bypass object checks.
//! ```compile_fail,E0603
//! use duplicity_core::openpgp::decode;
//! ```
//! ```compile_fail,E0603
//! use duplicity_core::openpgp::read_ciphertext;
//! ```
//! ```compile_fail,E0603
//! use duplicity_core::openpgp::DecodedObject;
//! ```
pub mod delta;
pub mod generated;
pub mod manifest_token;
pub mod path;
pub mod source;

pub mod manifest;
pub mod names;
pub mod signature;
mod verified;

pub mod envelope;
#[allow(
    dead_code,
    reason = "private native adapter awaits composed reader integration"
)]
pub(crate) mod openpgp;
pub mod tar;

pub mod multipart;

pub mod state;

pub mod selection;

pub mod chain;

pub mod restore_plan;

pub mod reader;

#[cfg(unix)]
pub mod local;

#[cfg(target_os = "linux")]
pub mod restore_linux;
