# ! [allow (warnings)] use vest_lib::combinators::mapped::spec::* ;
use vest_lib::combinators::* ;
use vest_lib::combinators::recursive::* ;
use Sum::Inl as L ;
use Sum::Inr as R ;
use vest_lib::Never ;
use vest_lib::core::exec::input::{
    InputBuf,
    InputSlice
}
;
use vest_lib::core::exec::output::OutputBuf ;
use vest_lib::core::exec::parser::* ;
use vest_lib::core::exec::serializer::* ;
use vest_lib::core::exec::ParseError ;
use vest_lib::core::exec::bytes_eq ;
use vest_lib::core::{
    proof::*,
    spec::*
}
;
use vest_lib::primitives::btcvarint::VarInt ;
use vest_lib::primitives::leb128::ULeb128 ;
use vstd::prelude::* ;
verus! {
// ============================================================
// Data Types
// ============================================================
# [doc = "data type for `hex_digit`."]
pub type HexDigit = u8 ;
pub type HexDigitSpec = u8 ;


# [doc = "data type for `escape`."]
pub type Escape = EscapeAnon ;
pub type EscapeSpec = EscapeAnonSpec ;


# [doc = "data type for `quoted_byte`."]
pub type QuotedByte = u8 ;
pub type QuotedByteSpec = u8 ;


# [doc = "data type for `quoted_atom`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub enum QuotedAtom {
    Plain (QuotedByte),
    Escaped (Escape),
}
# [verifier::ext_equal]
pub enum QuotedAtomSpec < T0 = QuotedByteSpec, T1 = EscapeSpec > {
    Plain (T0),
    Escaped (T1),
}
pub type QuotedAtomInner = Sum < QuotedByteSpec, EscapeSpec > ;
impl DeepView for QuotedAtom {
    type V = QuotedAtomSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        match self {
            QuotedAtom::Plain (v) => QuotedAtomSpec::Plain (v.deep_view()),
            QuotedAtom::Escaped (v) => QuotedAtomSpec::Escaped (v.deep_view()),
        }
    }
}
impl QuotedAtom {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view() == match self {
        QuotedAtom::Plain (v) => QuotedAtomSpec::Plain (v.deep_view()),
        QuotedAtom::Escaped (v) => QuotedAtomSpec::Escaped (v.deep_view()),
    }
   ,
    {
        reveal(< QuotedAtom as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > QuotedAtomSpec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: Sum < T0,
    T1 >) -> Self {
        match input {
            L (value) => Self::Plain (value),
            R (value) => Self::Escaped (value),
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> Sum < T0,
    T1 > {
        match self {
            Self::Plain (value) => L (value),
            Self::Escaped (value) => R (value),
        }
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(QuotedAtomSpec::from_structural) ;
        reveal(QuotedAtomSpec::into_structural) ;
        match self {
            Self::Plain (_) => {
            }
           ,
            Self::Escaped (_) => {
            }
           ,
        }
    }
    pub broadcast proof fn lemma_into_from (input: Sum < T0,
    T1 >) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(QuotedAtomSpec::from_structural) ;
        reveal(QuotedAtomSpec::into_structural) ;
        match input {
            L (_) => {
            }
           ,
            R (_) => {
            }
           ,
        }
    }
    pub proof fn lemma_into_structural_variant (self) ensures Self::into_structural (self) == match self {
        Self::Plain (value) => L (value),
        Self::Escaped (value) => R (value),
    }
   ,
    {
        reveal(QuotedAtomSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct QuotedAtomForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct QuotedAtomReverse ;
impl SpecMap for QuotedAtomForward {
    type Input = QuotedAtomInner ;
    type Output = QuotedAtomSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        QuotedAtomSpec::from_structural (input)
    }
}
impl SpecMap for QuotedAtomReverse {
    type Input = QuotedAtomSpec ;
    type Output = QuotedAtomInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `quoted_token`."]
# [derive (Debug, PartialEq, Eq, Clone)]
pub struct QuotedToken {
    pub opening: u8,
    pub atoms: Vec < QuotedAtom >,
    pub closing: u8,
}
# [verifier::ext_equal]
pub struct QuotedTokenSpec < T0 = u8, T1 = Seq < QuotedAtomSpec >, T2 = u8 > {
    pub opening: T0,
    pub atoms: T1,
    pub closing: T2,
}
pub type QuotedTokenInner = (u8, (Seq < QuotedAtomSpec >, u8)) ;
impl DeepView for QuotedToken {
    type V = QuotedTokenSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        QuotedTokenSpec {
            opening: self.opening.deep_view(),
            atoms: self.atoms.deep_view(),
            closing: self.closing.deep_view(),
        }
    }
}
impl QuotedToken {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().opening == self.opening.deep_view(),
    self.deep_view().atoms == self.atoms.deep_view(),
    self.deep_view().closing == self.closing.deep_view(),
    {
        reveal(< QuotedToken as DeepView>::deep_view) ;
    }
}
impl < T0, T1, T2 > QuotedTokenSpec < T0, T1, T2 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    (T1,
    T2))) -> Self {
        let (opening,
        (atoms,
        closing)) = input ;
        Self {
            opening,
            atoms,
            closing
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    (T1,
    T2)) {
        let Self {
            opening,
            atoms,
            closing
        }
        = self ;
        (opening,
        (atoms,
        closing))
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(QuotedTokenSpec::from_structural) ;
        reveal(QuotedTokenSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    (T1,
    T2))) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(QuotedTokenSpec::from_structural) ;
        reveal(QuotedTokenSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            opening,
            atoms,
            closing
        }
        => (opening,
        (atoms,
        closing)),
    }
   ,
    {
        reveal(QuotedTokenSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct QuotedTokenForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct QuotedTokenReverse ;
impl SpecMap for QuotedTokenForward {
    type Input = QuotedTokenInner ;
    type Output = QuotedTokenSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        QuotedTokenSpec::from_structural (input)
    }
}
impl SpecMap for QuotedTokenReverse {
    type Input = QuotedTokenSpec ;
    type Output = QuotedTokenInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `unquoted_byte`."]
pub type UnquotedByte = u8 ;
pub type UnquotedByteSpec = u8 ;


# [doc = "data type for `manifest_token`."]
pub type ManifestToken = QuotedToken ;
pub type ManifestTokenSpec = QuotedTokenSpec ;


# [doc = "data type for `escape_anon`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct EscapeAnon {
    pub high: HexDigit,
    pub low: HexDigit,
}
# [verifier::ext_equal]
pub struct EscapeAnonSpec < T0 = HexDigitSpec, T1 = HexDigitSpec > {
    pub high: T0,
    pub low: T1,
}
pub type EscapeAnonInner = (HexDigitSpec, HexDigitSpec) ;
impl DeepView for EscapeAnon {
    type V = EscapeAnonSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        EscapeAnonSpec {
            high: self.high.deep_view(),
            low: self.low.deep_view(),
        }
    }
}
impl EscapeAnon {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().high == self.high.deep_view(),
    self.deep_view().low == self.low.deep_view(),
    {
        reveal(< EscapeAnon as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > EscapeAnonSpec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (high,
        low) = input ;
        Self {
            high,
            low
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            high,
            low
        }
        = self ;
        (high,
        low)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(EscapeAnonSpec::from_structural) ;
        reveal(EscapeAnonSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(EscapeAnonSpec::from_structural) ;
        reveal(EscapeAnonSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            high,
            low
        }
        => (high,
        low),
    }
   ,
    {
        reveal(EscapeAnonSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct EscapeAnonForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct EscapeAnonReverse ;
impl SpecMap for EscapeAnonForward {
    type Input = EscapeAnonInner ;
    type Output = EscapeAnonSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        EscapeAnonSpec::from_structural (input)
    }
}
impl SpecMap for EscapeAnonReverse {
    type Input = EscapeAnonSpec ;
    type Output = EscapeAnonInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

// ============================================================
// Format Specifications
// ============================================================
# [doc = "named format combinator for `hex_digit`."]
# [derive (Clone, Copy)]
pub struct HexDigitFmt ;

pub type HexDigitFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl HexDigitFmt {
    # [doc = "specification constructor for `hex_digit`."] pub open spec fn spec_inner() -> HexDigitFmtSpec {
        Named ("hex_digit",
        Refined (U8,
        | x: u8 | x == 48 || x == 49 || x == 50 || x == 51 || x == 52 || x == 53 || x == 54 || x == 55 || x == 56 || x == 57 || x == 65 || x == 66 || x == 67 || x == 68 || x == 69 || x == 70 || x == 97 || x == 98 || x == 99 || x == 100 || x == 101 || x == 102))
    }
}


# [doc = "named format combinator for `escape`."]
# [derive (Clone, Copy)]
pub struct EscapeFmt ;

pub type EscapeFmtSpec = Named < PrefixTagged < U8, u8, PrefixTagged < U8, u8, EscapeAnonFmt > > > ;

impl EscapeFmt {
    # [doc = "specification constructor for `escape`."] pub open spec fn spec_inner() -> EscapeFmtSpec {
        Named ("escape",
        PrefixTagged (U8,
        92,
        PrefixTagged (U8,
        120,
        EscapeAnonFmt)))
    }
}


# [doc = "named format combinator for `quoted_byte`."]
# [derive (Clone, Copy)]
pub struct QuotedByteFmt ;

pub type QuotedByteFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl QuotedByteFmt {
    # [doc = "specification constructor for `quoted_byte`."] pub open spec fn spec_inner() -> QuotedByteFmtSpec {
        Named ("quoted_byte",
        Refined (U8,
        | x: u8 | ! (x == 0 || x == 10 || x == 13 || x == 34 || x == 92)))
    }
}


# [doc = "named format combinator for `quoted_atom`."]
# [derive (Clone, Copy)]
pub struct QuotedAtomFmt ;

pub type QuotedAtomFmtSpec = Named < Mapped < Choice < QuotedByteFmt, EscapeFmt >, BiMap < QuotedAtomForward, QuotedAtomReverse >> > ;

impl QuotedAtomFmt {
    # [doc = "specification constructor for `quoted_atom`."] pub open spec fn spec_inner() -> QuotedAtomFmtSpec {
        Named ("quoted_atom",
        Mapped {
            inner: Choice (QuotedByteFmt,
            EscapeFmt),
            mapper: BiMap (QuotedAtomForward,
            QuotedAtomReverse),
        }
        )
    }
}


# [doc = "named format combinator for `quoted_token`."]
# [derive (Clone, Copy)]
pub struct QuotedTokenFmt ;

pub type QuotedTokenFmtSpec = Named < Mapped < Pair < Const < U8, u8 >, Repeat < QuotedAtomFmt, Const < U8, u8 > > >, BiMap < QuotedTokenForward, QuotedTokenReverse >> > ;

impl QuotedTokenFmt {
    # [doc = "specification constructor for `quoted_token`."] pub open spec fn spec_inner() -> QuotedTokenFmtSpec {
        Named ("quoted_token",
        Mapped {
            inner: Pair (Const (U8,
            34),
            Repeat (QuotedAtomFmt,
            Const (U8,
            34))),
            mapper: BiMap (QuotedTokenForward,
            QuotedTokenReverse),
        }
        )
    }
}


# [doc = "named format combinator for `unquoted_byte`."]
# [derive (Clone, Copy)]
pub struct UnquotedByteFmt ;

pub type UnquotedByteFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl UnquotedByteFmt {
    # [doc = "specification constructor for `unquoted_byte`."] pub open spec fn spec_inner() -> UnquotedByteFmtSpec {
        Named ("unquoted_byte",
        Refined (U8,
        | x: u8 | ! (x == 0 || x == 9 || x == 10 || x == 11 || x == 12 || x == 13 || x == 32 || x == 34 || x == 39 || x == 92)))
    }
}


# [doc = "named format combinator for `manifest_token`."]
# [derive (Clone, Copy)]
pub struct ManifestTokenFmt ;

pub type ManifestTokenFmtSpec = Named < QuotedTokenFmt > ;

impl ManifestTokenFmt {
    # [doc = "specification constructor for `manifest_token`."] pub open spec fn spec_inner() -> ManifestTokenFmtSpec {
        Named ("manifest_token",
        QuotedTokenFmt)
    }
}


# [doc = "named format combinator for `escape_anon`."]
# [derive (Clone, Copy)]
pub struct EscapeAnonFmt ;

pub type EscapeAnonFmtSpec = Named < Mapped < Pair < HexDigitFmt, HexDigitFmt >, BiMap < EscapeAnonForward, EscapeAnonReverse >> > ;

impl EscapeAnonFmt {
    # [doc = "specification constructor for `escape_anon`."] pub open spec fn spec_inner() -> EscapeAnonFmtSpec {
        Named ("escape_anon",
        Mapped {
            inner: Pair (HexDigitFmt,
            HexDigitFmt),
            mapper: BiMap (EscapeAnonForward,
            EscapeAnonReverse),
        }
        )
    }
}

// ============================================================
// Derived Parser, Serializer, Length, and Consistency Specifications
// ============================================================
mod derived_specs {
    use super::*;

    impl SpecParser for HexDigitFmt {
        type PVal = HexDigitSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for HexDigitFmt {
        type Val = HexDigitSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for HexDigitFmt {
        type SValue = HexDigitSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for HexDigitFmt {
        type SVal = HexDigitSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for HexDigitFmt {
        type T = HexDigitSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for EscapeFmt {
        type PVal = EscapeSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for EscapeFmt {
        type Val = EscapeSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for EscapeFmt {
        type SValue = EscapeSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for EscapeFmt {
        type SVal = EscapeSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for EscapeFmt {
        type T = EscapeSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for QuotedByteFmt {
        type PVal = QuotedByteSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for QuotedByteFmt {
        type Val = QuotedByteSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for QuotedByteFmt {
        type SValue = QuotedByteSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for QuotedByteFmt {
        type SVal = QuotedByteSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for QuotedByteFmt {
        type T = QuotedByteSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for QuotedAtomFmt {
        type PVal = QuotedAtomSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for QuotedAtomFmt {
        type Val = QuotedAtomSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for QuotedAtomFmt {
        type SValue = QuotedAtomSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for QuotedAtomFmt {
        type SVal = QuotedAtomSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for QuotedAtomFmt {
        type T = QuotedAtomSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for QuotedTokenFmt {
        type PVal = QuotedTokenSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for QuotedTokenFmt {
        type Val = QuotedTokenSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for QuotedTokenFmt {
        type SValue = QuotedTokenSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for QuotedTokenFmt {
        type SVal = QuotedTokenSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for QuotedTokenFmt {
        type T = QuotedTokenSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for UnquotedByteFmt {
        type PVal = UnquotedByteSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for UnquotedByteFmt {
        type Val = UnquotedByteSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for UnquotedByteFmt {
        type SValue = UnquotedByteSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for UnquotedByteFmt {
        type SVal = UnquotedByteSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for UnquotedByteFmt {
        type T = UnquotedByteSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for ManifestTokenFmt {
        type PVal = ManifestTokenSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for ManifestTokenFmt {
        type Val = ManifestTokenSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for ManifestTokenFmt {
        type SValue = ManifestTokenSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for ManifestTokenFmt {
        type SVal = ManifestTokenSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for ManifestTokenFmt {
        type T = ManifestTokenSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for EscapeAnonFmt {
        type PVal = EscapeAnonSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for EscapeAnonFmt {
        type Val = EscapeAnonSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for EscapeAnonFmt {
        type SValue = EscapeAnonSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for EscapeAnonFmt {
        type SVal = EscapeAnonSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for EscapeAnonFmt {
        type T = EscapeAnonSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }
}

// ============================================================
// Proven Format Properties
// ============================================================
mod derived_proofs {
    use super::*;
    broadcast use {
        crate::verified::token_refinement::group_token_facts,
        vest_lib::combinators::disjoint::disjointness_lemmas,
        QuotedAtomSpec::lemma_from_into,
        QuotedAtomSpec::lemma_into_from,
        QuotedTokenSpec::lemma_from_into,
        QuotedTokenSpec::lemma_into_from,
        EscapeAnonSpec::lemma_from_into,
        EscapeAnonSpec::lemma_into_from,
    };

    impl SafeParser for HexDigitFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for HexDigitFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for HexDigitFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            reveal(< HexDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            reveal(< HexDigitFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for HexDigitFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< HexDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for HexDigitFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< HexDigitFmt as SpecSerializer>::spec_serialize) ;
            reveal(< HexDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for HexDigitFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            reveal(< HexDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< HexDigitFmt as Consistency>::consistent) ;
            reveal(< HexDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for HexDigitFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for HexDigitFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< HexDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< HexDigitFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for HexDigitFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< HexDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< HexDigitFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for EscapeFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for EscapeFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for EscapeFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for EscapeFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for EscapeFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< EscapeFmt as SpecSerializer>::spec_serialize) ;
            reveal(< EscapeFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for EscapeFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeFmt as Consistency>::consistent) ;
            reveal(< EscapeFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for EscapeFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< EscapeFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for EscapeFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< EscapeFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for EscapeFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< EscapeFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for QuotedByteFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for QuotedByteFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for QuotedByteFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedByteFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for QuotedByteFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for QuotedByteFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< QuotedByteFmt as SpecSerializer>::spec_serialize) ;
            reveal(< QuotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for QuotedByteFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedByteFmt as Consistency>::consistent) ;
            reveal(< QuotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for QuotedByteFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for QuotedByteFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< QuotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedByteFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for QuotedByteFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< QuotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedByteFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for QuotedAtomFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for QuotedAtomFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for QuotedAtomFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedAtomFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedAtomInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedAtomSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedAtomFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedAtomInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedAtomSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for QuotedAtomFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedAtomFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for QuotedAtomFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< QuotedAtomFmt as SpecSerializer>::spec_serialize) ;
            reveal(< QuotedAtomFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for QuotedAtomFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedAtomFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedAtomFmt as Consistency>::consistent) ;
            reveal(< QuotedAtomFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: QuotedAtomSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                QuotedAtomSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for QuotedAtomFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedAtomInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedAtomSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for QuotedAtomFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< QuotedAtomFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedAtomFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for QuotedAtomFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< QuotedAtomFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedAtomFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for QuotedTokenFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for QuotedTokenFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for QuotedTokenFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedTokenInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedTokenSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedTokenFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedTokenInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedTokenSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for QuotedTokenFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for QuotedTokenFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< QuotedTokenFmt as SpecSerializer>::spec_serialize) ;
            reveal(< QuotedTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for QuotedTokenFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            reveal(< QuotedTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedTokenFmt as Consistency>::consistent) ;
            reveal(< QuotedTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: QuotedTokenSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                QuotedTokenSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for QuotedTokenFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: QuotedTokenInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                QuotedTokenSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for QuotedTokenFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< QuotedTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedTokenFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for QuotedTokenFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< QuotedTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< QuotedTokenFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for UnquotedByteFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for UnquotedByteFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for UnquotedByteFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< UnquotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< UnquotedByteFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for UnquotedByteFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< UnquotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for UnquotedByteFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< UnquotedByteFmt as SpecSerializer>::spec_serialize) ;
            reveal(< UnquotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for UnquotedByteFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            reveal(< UnquotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< UnquotedByteFmt as Consistency>::consistent) ;
            reveal(< UnquotedByteFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for UnquotedByteFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for UnquotedByteFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< UnquotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< UnquotedByteFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for UnquotedByteFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< UnquotedByteFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< UnquotedByteFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for ManifestTokenFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for ManifestTokenFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for ManifestTokenFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            reveal(< ManifestTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            reveal(< ManifestTokenFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for ManifestTokenFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< ManifestTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for ManifestTokenFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< ManifestTokenFmt as SpecSerializer>::spec_serialize) ;
            reveal(< ManifestTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for ManifestTokenFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            reveal(< ManifestTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< ManifestTokenFmt as Consistency>::consistent) ;
            reveal(< ManifestTokenFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for ManifestTokenFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for ManifestTokenFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< ManifestTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< ManifestTokenFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for ManifestTokenFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< ManifestTokenFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< ManifestTokenFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for EscapeAnonFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for EscapeAnonFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for EscapeAnonFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeAnonFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: EscapeAnonInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                EscapeAnonSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeAnonFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: EscapeAnonInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                EscapeAnonSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for EscapeAnonFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeAnonFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for EscapeAnonFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< EscapeAnonFmt as SpecSerializer>::spec_serialize) ;
            reveal(< EscapeAnonFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for EscapeAnonFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            reveal(< EscapeAnonFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeAnonFmt as Consistency>::consistent) ;
            reveal(< EscapeAnonFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: EscapeAnonSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                EscapeAnonSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for EscapeAnonFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: EscapeAnonInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                EscapeAnonSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for EscapeAnonFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< EscapeAnonFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeAnonFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for EscapeAnonFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< EscapeAnonFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< EscapeAnonFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }
}

// ============================================================
// Executable Implementations
// ============================================================
mod exec_impls {
broadcast use crate::verified::token_refinement::group_token_facts;

    use super::*;

    impl<'i> Parser<&'i [u8]> for HexDigitFmt {
        type PT = HexDigit;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<HexDigitFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(v == 48 || v == 49 || v == 50 || v == 51 || v == 52 || v == 53 || v == 54 || v == 55 || v == 56 || v == 57 || v == 65 || v == 66 || v == 67 || v == 68 || v == 69 || v == 70 || v == 97 || v == 98 || v == 99 || v == 100 || v == 101 || v == 102) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, HexDigit> for HexDigitFmt {
        fn serialize_into(&self, v: &HexDigit, obuf: &mut Output) {
            reveal(<HexDigitFmt as SpecSerializer>::spec_serialize);
            reveal(<HexDigitFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<HexDigit> for HexDigitFmt {
        fn prepare(&self, v: &HexDigit) -> Result<usize, PreSerializeError> {
            reveal(<HexDigitFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 48 || *v == 49 || *v == 50 || *v == 51 || *v == 52 || *v == 53 || *v == 54 || *v == 55 || *v == 56 || *v == 57 || *v == 65 || *v == 66 || *v == 67 || *v == 68 || *v == 69 || *v == 70 || *v == 97 || *v == 98 || *v == 99 || *v == 100 || *v == 101 || *v == 102) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for EscapeFmt {
        type PT = Escape;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<EscapeFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = PrefixTagged (U8, 92, PrefixTagged (U8, 120, EscapeAnonFmt))
            .parse(ibuf)?;
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Escape> for EscapeFmt {
        fn serialize_into(&self, v: &Escape, obuf: &mut Output) {
            reveal(<EscapeFmt as SpecSerializer>::spec_serialize);
            reveal(<EscapeFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            PrefixTagged (U8, 92, PrefixTagged (U8, 120, EscapeAnonFmt))
            .serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Escape> for EscapeFmt {
        fn prepare(&self, v: &Escape) -> Result<usize, PreSerializeError> {
            reveal(<EscapeFmt as SpecByteLen>::byte_len);
            (PrefixTagged (U8, 92, PrefixTagged (U8, 120, EscapeAnonFmt))).prepare (v)
        }
    }



    impl<'i> Parser<&'i [u8]> for QuotedByteFmt {
        type PT = QuotedByte;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<QuotedByteFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(! (v == 0 || v == 10 || v == 13 || v == 34 || v == 92)) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, QuotedByte> for QuotedByteFmt {
        fn serialize_into(&self, v: &QuotedByte, obuf: &mut Output) {
            reveal(<QuotedByteFmt as SpecSerializer>::spec_serialize);
            reveal(<QuotedByteFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<QuotedByte> for QuotedByteFmt {
        fn prepare(&self, v: &QuotedByte) -> Result<usize, PreSerializeError> {
            reveal(<QuotedByteFmt as SpecByteLen>::byte_len);
            {
        if ! (! (*v == 0 || *v == 10 || *v == 13 || *v == 34 || *v == 92)) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for QuotedAtomFmt {
        type PT = QuotedAtom;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<QuotedAtomFmt as SpecParser>::spec_parse);
            reveal(<QuotedAtom as DeepView>::deep_view);
            reveal(QuotedAtomSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = match (Named ("quoted_byte", QuotedByteFmt)).parse (& rest) {
        Ok ((n,
        va)) if ! (va == 0 || va == 10 || va == 13 || va == 34 || va == 92) => {
            Ok ((n,
            QuotedAtom::Plain (va)))
        }
       ,
        _ => match (Named ("escape",
        EscapeFmt)).parse (& rest) {
            Ok ((n,
            va)) => {
                Ok ((n,
                QuotedAtom::Escaped (va)))
            }
           ,
            _ => Err (ParseError::invalid_choice()),
        }
       ,
    }
    ?;
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, QuotedAtom> for QuotedAtomFmt {
        fn serialize_into(&self, v: &QuotedAtom, obuf: &mut Output) {
            reveal(<QuotedAtomFmt as SpecSerializer>::spec_serialize);
            reveal(<QuotedAtomFmt as SpecByteLen>::byte_len);
            reveal(<QuotedAtom as DeepView>::deep_view);
            reveal(QuotedAtomSpec::into_structural);
            let ghost old_obuf = obuf@;

            match v {
                QuotedAtom::Plain (v) => {
                    (QuotedByteFmt).serialize_into (v,
                    obuf) ;
                }
                ,
                QuotedAtom::Escaped (v) => {
                    (EscapeFmt).serialize_into (v,
                    obuf) ;
                }
                ,
            }

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<QuotedAtom> for QuotedAtomFmt {
        fn prepare(&self, v: &QuotedAtom) -> Result<usize, PreSerializeError> {
            reveal(<QuotedAtomFmt as SpecByteLen>::byte_len);
            reveal(<QuotedAtom as DeepView>::deep_view);
            reveal(QuotedAtomSpec::into_structural);
            match v {
                QuotedAtom::Plain (v) => {
                    if ! (! (*v == 0 || *v == 10 || *v == 13 || *v == 34 || *v == 92)) {
                        Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                    }
                    else {
                        (Named ("quoted_byte",
                        QuotedByteFmt)).prepare (v)
                    }
                }
                ,
                QuotedAtom::Escaped (v) => (Named ("escape", EscapeFmt)).prepare (v),
            }
        }
    }



    impl<'i> Parser<&'i [u8]> for QuotedTokenFmt {
        type PT = QuotedToken;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<QuotedTokenFmt as SpecParser>::spec_parse);
            reveal(<QuotedToken as DeepView>::deep_view);
            reveal(QuotedTokenSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, opening) = Const (U8, 34).parse(&rest)?;
            let rest = rest.skip(n1);
            let (n2, atoms) = (Star (QuotedAtomFmt)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let (n3, closing) = Const (U8, 34).parse(&rest)?;
            let rest = rest.skip(n3);
            let total_n = n1 + n2 + n3;
            let final_v = QuotedToken {
                opening,
                atoms,
                closing,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, QuotedToken> for QuotedTokenFmt {
        fn serialize_into(&self, v: &QuotedToken, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<QuotedTokenFmt as SpecSerializer>::spec_serialize);
            reveal(<QuotedTokenFmt as SpecByteLen>::byte_len);
            reveal(<QuotedToken as DeepView>::deep_view);
            reveal(QuotedTokenSpec::into_structural);
            let ghost old_obuf = obuf@;

            let QuotedToken {
                opening,
                atoms,
                closing,
            } = v;
            U8.serialize_into(opening, obuf);
            Star (QuotedAtomFmt).serialize_into(atoms, obuf);
            U8.serialize_into(closing, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<QuotedToken> for QuotedTokenFmt {
        fn prepare(&self, v: &QuotedToken) -> Result<usize, PreSerializeError> {
            reveal(<QuotedTokenFmt as SpecByteLen>::byte_len);
            reveal(<QuotedToken as DeepView>::deep_view);
            reveal(QuotedTokenSpec::into_structural);
            let QuotedToken {
                opening,
                atoms,
                closing,
            } = v;
            let l1 = (Const (U8, 34)).prepare (opening) ?;
            let l2 = (Star (QuotedAtomFmt)).prepare (atoms) ?;
            let l3 = (Const (U8, 34)).prepare (closing) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l3).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for UnquotedByteFmt {
        type PT = UnquotedByte;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<UnquotedByteFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(! (v == 0 || v == 9 || v == 10 || v == 11 || v == 12 || v == 13 || v == 32 || v == 34 || v == 39 || v == 92)) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, UnquotedByte> for UnquotedByteFmt {
        fn serialize_into(&self, v: &UnquotedByte, obuf: &mut Output) {
            reveal(<UnquotedByteFmt as SpecSerializer>::spec_serialize);
            reveal(<UnquotedByteFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<UnquotedByte> for UnquotedByteFmt {
        fn prepare(&self, v: &UnquotedByte) -> Result<usize, PreSerializeError> {
            reveal(<UnquotedByteFmt as SpecByteLen>::byte_len);
            {
        if ! (! (*v == 0 || *v == 9 || *v == 10 || *v == 11 || *v == 12 || *v == 13 || *v == 32 || *v == 34 || *v == 39 || *v == 92)) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for ManifestTokenFmt {
        type PT = ManifestToken;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<ManifestTokenFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = Named ("quoted_token", QuotedTokenFmt).parse(ibuf)?;
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, ManifestToken> for ManifestTokenFmt {
        fn serialize_into(&self, v: &ManifestToken, obuf: &mut Output) {
            reveal(<ManifestTokenFmt as SpecSerializer>::spec_serialize);
            reveal(<ManifestTokenFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            QuotedTokenFmt.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<ManifestToken> for ManifestTokenFmt {
        fn prepare(&self, v: &ManifestToken) -> Result<usize, PreSerializeError> {
            reveal(<ManifestTokenFmt as SpecByteLen>::byte_len);
            Named ("quoted_token", QuotedTokenFmt).prepare(v)
        }
    }



    impl<'i> Parser<&'i [u8]> for EscapeAnonFmt {
        type PT = EscapeAnon;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<EscapeAnonFmt as SpecParser>::spec_parse);
            reveal(<EscapeAnon as DeepView>::deep_view);
            reveal(EscapeAnonSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, high) = (Named ("hex_digit", HexDigitFmt)).parse (& rest) ?;
            if !(high == 48 || high == 49 || high == 50 || high == 51 || high == 52 || high == 53 || high == 54 || high == 55 || high == 56 || high == 57 || high == 65 || high == 66 || high == 67 || high == 68 || high == 69 || high == 70 || high == 97 || high == 98 || high == 99 || high == 100 || high == 101 || high == 102) {
                return Err(ParseError::predicate_failed());
            }
            let rest = rest.skip(n1);
            let (n2, low) = (Named ("hex_digit", HexDigitFmt)).parse (& rest) ?;
            if !(low == 48 || low == 49 || low == 50 || low == 51 || low == 52 || low == 53 || low == 54 || low == 55 || low == 56 || low == 57 || low == 65 || low == 66 || low == 67 || low == 68 || low == 69 || low == 70 || low == 97 || low == 98 || low == 99 || low == 100 || low == 101 || low == 102) {
                return Err(ParseError::predicate_failed());
            }
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = EscapeAnon {
                high,
                low,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, EscapeAnon> for EscapeAnonFmt {
        fn serialize_into(&self, v: &EscapeAnon, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<EscapeAnonFmt as SpecSerializer>::spec_serialize);
            reveal(<EscapeAnonFmt as SpecByteLen>::byte_len);
            reveal(<EscapeAnon as DeepView>::deep_view);
            reveal(EscapeAnonSpec::into_structural);
            let ghost old_obuf = obuf@;

            let EscapeAnon {
                high,
                low,
            } = v;
            HexDigitFmt.serialize_into(high, obuf);
            HexDigitFmt.serialize_into(low, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<EscapeAnon> for EscapeAnonFmt {
        fn prepare(&self, v: &EscapeAnon) -> Result<usize, PreSerializeError> {
            reveal(<EscapeAnonFmt as SpecByteLen>::byte_len);
            reveal(<EscapeAnon as DeepView>::deep_view);
            reveal(EscapeAnonSpec::into_structural);
            let EscapeAnon {
                high,
                low,
            } = v;
            let l1 = {
                if ! (* high == 48 || * high == 49 || * high == 50 || * high == 51 || * high == 52 || * high == 53 || * high == 54 || * high == 55 || * high == 56 || * high == 57 || * high == 65 || * high == 66 || * high == 67 || * high == 68 || * high == 69 || * high == 70 || * high == 97 || * high == 98 || * high == 99 || * high == 100 || * high == 101 || * high == 102) {
                    Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                }
                else {
                    (Named ("hex_digit",
                    HexDigitFmt)).prepare (high)
                }
            }
            ?;
            let l2 = {
                if ! (* low == 48 || * low == 49 || * low == 50 || * low == 51 || * low == 52 || * low == 53 || * low == 54 || * low == 55 || * low == 56 || * low == 57 || * low == 65 || * low == 66 || * low == 67 || * low == 68 || * low == 69 || * low == 70 || * low == 97 || * low == 98 || * low == 99 || * low == 100 || * low == 101 || * low == 102) {
                    Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                }
                else {
                    (Named ("hex_digit",
                    HexDigitFmt)).prepare (low)
                }
            }
            ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }

}
}
