//! Executable parsing, validation, length calculation, and serialization.
//!
//! Application code normally uses the following trait methods:
//!
//! - [`Parser::parse`] consumes an input prefix and produces a structured value;
//! - [`Prepare::prepare`] validates a value against the format specification and computes its exact wire length;
//! - [`SerializerExt::serialize`] writes into an exactly sized slice without allocation.
//!
//! [`InputBuf`] and [`OutputBuf`] let combinators share implementations across different buffer types.
pub mod bridge_lemmas;
pub mod error;
pub mod fns;
pub mod input;
pub mod output;
pub mod parser;
pub mod serializer;

pub use error::{ParseError, ParseErrorKind};
pub use input::{InputBuf, InputSlice};
pub use output::{OutputBuf, OutputSlice};
pub use parser::{PResult, Parser};
pub use serializer::{
    ByteLen, ComplianceErrorKind, PreSerializeError, Prepare, Serializer, SerializerExt,
};

use vstd::prelude::*;

verus! {

#[inline(always)]
pub fn bytes_eq(a: &[u8], b: &[u8]) -> (r: bool)
    ensures
        r == (a@ == b@),
        r == (a.deep_view() == b.deep_view()),
{
    let res = *a == *b;
    assert(a@ == a.deep_view());
    assert(b@ == b.deep_view());
    assert(res ==> (a.deep_view() == b.deep_view()));
    res
}

} // verus!
