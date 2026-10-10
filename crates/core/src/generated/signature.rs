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
# [doc = "data type for `signature_magic`."]
pub type SignatureMagic = u32 ;
pub type SignatureMagicSpec = u32 ;


# [doc = "data type for `signature_header`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct SignatureHeader {
    pub magic: SignatureMagic,
    pub block_length: u32,
    pub strong_length: u32,
}
# [verifier::ext_equal]
pub struct SignatureHeaderSpec < T0 = SignatureMagicSpec, T1 = u32, T2 = u32 > {
    pub magic: T0,
    pub block_length: T1,
    pub strong_length: T2,
}
pub type SignatureHeaderInner = (SignatureMagicSpec, (u32, u32)) ;
impl DeepView for SignatureHeader {
    type V = SignatureHeaderSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        SignatureHeaderSpec {
            magic: self.magic.deep_view(),
            block_length: self.block_length.deep_view(),
            strong_length: self.strong_length.deep_view(),
        }
    }
}
impl SignatureHeader {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().magic == self.magic.deep_view(),
    self.deep_view().block_length == self.block_length.deep_view(),
    self.deep_view().strong_length == self.strong_length.deep_view(),
    {
        reveal(< SignatureHeader as DeepView>::deep_view) ;
    }
}
impl < T0, T1, T2 > SignatureHeaderSpec < T0, T1, T2 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    (T1,
    T2))) -> Self {
        let (magic,
        (block_length,
        strong_length)) = input ;
        Self {
            magic,
            block_length,
            strong_length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    (T1,
    T2)) {
        let Self {
            magic,
            block_length,
            strong_length
        }
        = self ;
        (magic,
        (block_length,
        strong_length))
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(SignatureHeaderSpec::from_structural) ;
        reveal(SignatureHeaderSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    (T1,
    T2))) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(SignatureHeaderSpec::from_structural) ;
        reveal(SignatureHeaderSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            magic,
            block_length,
            strong_length
        }
        => (magic,
        (block_length,
        strong_length)),
    }
   ,
    {
        reveal(SignatureHeaderSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct SignatureHeaderForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct SignatureHeaderReverse ;
impl SpecMap for SignatureHeaderForward {
    type Input = SignatureHeaderInner ;
    type Output = SignatureHeaderSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        SignatureHeaderSpec::from_structural (input)
    }
}
impl SpecMap for SignatureHeaderReverse {
    type Input = SignatureHeaderSpec ;
    type Output = SignatureHeaderInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

// ============================================================
// Format Specifications
// ============================================================
# [doc = "named format combinator for `signature_magic`."]
# [derive (Clone, Copy)]
pub struct SignatureMagicFmt ;

pub type SignatureMagicFmtSpec = Named < Refined < U32Be, PredFnSpec < u32 >> > ;

impl SignatureMagicFmt {
    # [doc = "specification constructor for `signature_magic`."] pub open spec fn spec_inner() -> SignatureMagicFmtSpec {
        Named ("signature_magic",
        Refined (U32Be,
        | x: u32 | x == 1920139574 || x == 1920139575 || x == 1920139590 || x == 1920139591))
    }
}


# [doc = "named format combinator for `signature_header`."]
# [derive (Clone, Copy)]
pub struct SignatureHeaderFmt ;

pub type SignatureHeaderFmtSpec = Named < Mapped < Pair < SignatureMagicFmt, Pair < U32Be, U32Be > >, BiMap < SignatureHeaderForward, SignatureHeaderReverse >> > ;

impl SignatureHeaderFmt {
    # [doc = "specification constructor for `signature_header`."] pub open spec fn spec_inner() -> SignatureHeaderFmtSpec {
        Named ("signature_header",
        Mapped {
            inner: Pair (SignatureMagicFmt,
            Pair (U32Be,
            U32Be)),
            mapper: BiMap (SignatureHeaderForward,
            SignatureHeaderReverse),
        }
        )
    }
}

// ============================================================
// Derived Parser, Serializer, Length, and Consistency Specifications
// ============================================================
mod derived_specs {
    use super::*;

    impl SpecParser for SignatureMagicFmt {
        type PVal = SignatureMagicSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for SignatureMagicFmt {
        type Val = SignatureMagicSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for SignatureMagicFmt {
        type SValue = SignatureMagicSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for SignatureMagicFmt {
        type SVal = SignatureMagicSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for SignatureMagicFmt {
        type T = SignatureMagicSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for SignatureHeaderFmt {
        type PVal = SignatureHeaderSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for SignatureHeaderFmt {
        type Val = SignatureHeaderSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for SignatureHeaderFmt {
        type SValue = SignatureHeaderSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for SignatureHeaderFmt {
        type SVal = SignatureHeaderSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for SignatureHeaderFmt {
        type T = SignatureHeaderSpec ;
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
        vest_lib::combinators::disjoint::disjointness_lemmas,
        SignatureHeaderSpec::lemma_from_into,
        SignatureHeaderSpec::lemma_into_from,
    };

    impl SafeParser for SignatureMagicFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for SignatureMagicFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for SignatureMagicFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureMagicFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for SignatureMagicFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for SignatureMagicFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< SignatureMagicFmt as SpecSerializer>::spec_serialize) ;
            reveal(< SignatureMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for SignatureMagicFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureMagicFmt as Consistency>::consistent) ;
            reveal(< SignatureMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for SignatureMagicFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for SignatureMagicFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< SignatureMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureMagicFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for SignatureMagicFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< SignatureMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureMagicFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for SignatureHeaderFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for SignatureHeaderFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for SignatureHeaderFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: SignatureHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                SignatureHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureHeaderFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: SignatureHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                SignatureHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for SignatureHeaderFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for SignatureHeaderFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< SignatureHeaderFmt as SpecSerializer>::spec_serialize) ;
            reveal(< SignatureHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for SignatureHeaderFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< SignatureHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureHeaderFmt as Consistency>::consistent) ;
            reveal(< SignatureHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: SignatureHeaderSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                SignatureHeaderSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for SignatureHeaderFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: SignatureHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                SignatureHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for SignatureHeaderFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< SignatureHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureHeaderFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for SignatureHeaderFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< SignatureHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< SignatureHeaderFmt as SpecSerializer>::spec_serialize) ;
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
    use super::*;

    impl<'i> Parser<&'i [u8]> for SignatureMagicFmt {
        type PT = SignatureMagic;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<SignatureMagicFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U32Be.parse(ibuf)?;
            if !(v == 1920139574 || v == 1920139575 || v == 1920139590 || v == 1920139591) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, SignatureMagic> for SignatureMagicFmt {
        fn serialize_into(&self, v: &SignatureMagic, obuf: &mut Output) {
            reveal(<SignatureMagicFmt as SpecSerializer>::spec_serialize);
            reveal(<SignatureMagicFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U32Be.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<SignatureMagic> for SignatureMagicFmt {
        fn prepare(&self, v: &SignatureMagic) -> Result<usize, PreSerializeError> {
            reveal(<SignatureMagicFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 1920139574 || *v == 1920139575 || *v == 1920139590 || *v == 1920139591) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U32Be).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for SignatureHeaderFmt {
        type PT = SignatureHeader;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<SignatureHeaderFmt as SpecParser>::spec_parse);
            reveal(<SignatureHeader as DeepView>::deep_view);
            reveal(SignatureHeaderSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, magic) = (Named ("signature_magic", SignatureMagicFmt)).parse (& rest) ?;
            if !(magic == 1920139574 || magic == 1920139575 || magic == 1920139590 || magic == 1920139591) {
                return Err(ParseError::predicate_failed());
            }
            let rest = rest.skip(n1);
            let (n2, block_length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let (n3, strong_length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n3);
            let total_n = n1 + n2 + n3;
            let final_v = SignatureHeader {
                magic,
                block_length,
                strong_length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, SignatureHeader> for SignatureHeaderFmt {
        fn serialize_into(&self, v: &SignatureHeader, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<SignatureHeaderFmt as SpecSerializer>::spec_serialize);
            reveal(<SignatureHeaderFmt as SpecByteLen>::byte_len);
            reveal(<SignatureHeader as DeepView>::deep_view);
            reveal(SignatureHeaderSpec::into_structural);
            let ghost old_obuf = obuf@;

            let SignatureHeader {
                magic,
                block_length,
                strong_length,
            } = v;
            SignatureMagicFmt.serialize_into(magic, obuf);
            U32Be.serialize_into(block_length, obuf);
            U32Be.serialize_into(strong_length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<SignatureHeader> for SignatureHeaderFmt {
        fn prepare(&self, v: &SignatureHeader) -> Result<usize, PreSerializeError> {
            reveal(<SignatureHeaderFmt as SpecByteLen>::byte_len);
            reveal(<SignatureHeader as DeepView>::deep_view);
            reveal(SignatureHeaderSpec::into_structural);
            let SignatureHeader {
                magic,
                block_length,
                strong_length,
            } = v;
            let l1 = {
                if ! (* magic == 1920139574 || * magic == 1920139575 || * magic == 1920139590 || * magic == 1920139591) {
                    Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                }
                else {
                    (Named ("signature_magic",
                    SignatureMagicFmt)).prepare (magic)
                }
            }
            ?;
            let l2 = (U32Be).prepare (block_length) ?;
            let l3 = (U32Be).prepare (strong_length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l3).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }

}
}
