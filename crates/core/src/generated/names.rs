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
# [doc = "data type for `date_digit`."]
pub type DateDigit = u8 ;
pub type DateDigitSpec = u8 ;


# [doc = "data type for `date_t`."]
pub type DateT = u8 ;
pub type DateTSpec = u8 ;


# [doc = "data type for `date_z`."]
pub type DateZ = u8 ;
pub type DateZSpec = u8 ;


# [doc = "data type for `canonical_timestamp`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct CanonicalTimestamp {
    pub year: [DateDigit ;
    4],
    pub month: [DateDigit ;
    2],
    pub day: [DateDigit ;
    2],
    pub separator: DateT,
    pub hour: [DateDigit ;
    2],
    pub minute: [DateDigit ;
    2],
    pub second: [DateDigit ;
    2],
    pub zone: DateZ,
}
# [verifier::ext_equal]
pub struct CanonicalTimestampSpec < T0 = Seq < DateDigitSpec >, T1 = Seq < DateDigitSpec >, T2 = Seq < DateDigitSpec >, T3 = DateTSpec, T4 = Seq < DateDigitSpec >, T5 = Seq < DateDigitSpec >, T6 = Seq < DateDigitSpec >, T7 = DateZSpec > {
    pub year: T0,
    pub month: T1,
    pub day: T2,
    pub separator: T3,
    pub hour: T4,
    pub minute: T5,
    pub second: T6,
    pub zone: T7,
}
pub type CanonicalTimestampInner = (Seq < DateDigitSpec >, (Seq < DateDigitSpec >, (Seq < DateDigitSpec >, (DateTSpec, (Seq < DateDigitSpec >, (Seq < DateDigitSpec >, (Seq < DateDigitSpec >, DateZSpec))))))) ;
impl DeepView for CanonicalTimestamp {
    type V = CanonicalTimestampSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        CanonicalTimestampSpec {
            year: self.year.deep_view(),
            month: self.month.deep_view(),
            day: self.day.deep_view(),
            separator: self.separator.deep_view(),
            hour: self.hour.deep_view(),
            minute: self.minute.deep_view(),
            second: self.second.deep_view(),
            zone: self.zone.deep_view(),
        }
    }
}
impl CanonicalTimestamp {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().year == self.year.deep_view(),
    self.deep_view().month == self.month.deep_view(),
    self.deep_view().day == self.day.deep_view(),
    self.deep_view().separator == self.separator.deep_view(),
    self.deep_view().hour == self.hour.deep_view(),
    self.deep_view().minute == self.minute.deep_view(),
    self.deep_view().second == self.second.deep_view(),
    self.deep_view().zone == self.zone.deep_view(),
    {
        reveal(< CanonicalTimestamp as DeepView>::deep_view) ;
    }
}
impl < T0, T1, T2, T3, T4, T5, T6, T7 > CanonicalTimestampSpec < T0, T1, T2, T3, T4, T5, T6, T7 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    T7)))))))) -> Self {
        let (year,
        (month,
        (day,
        (separator,
        (hour,
        (minute,
        (second,
        zone))))))) = input ;
        Self {
            year,
            month,
            day,
            separator,
            hour,
            minute,
            second,
            zone
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    T7))))))) {
        let Self {
            year,
            month,
            day,
            separator,
            hour,
            minute,
            second,
            zone
        }
        = self ;
        (year,
        (month,
        (day,
        (separator,
        (hour,
        (minute,
        (second,
        zone)))))))
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(CanonicalTimestampSpec::from_structural) ;
        reveal(CanonicalTimestampSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    T7)))))))) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(CanonicalTimestampSpec::from_structural) ;
        reveal(CanonicalTimestampSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            year,
            month,
            day,
            separator,
            hour,
            minute,
            second,
            zone
        }
        => (year,
        (month,
        (day,
        (separator,
        (hour,
        (minute,
        (second,
        zone))))))),
    }
   ,
    {
        reveal(CanonicalTimestampSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CanonicalTimestampForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CanonicalTimestampReverse ;
impl SpecMap for CanonicalTimestampForward {
    type Input = CanonicalTimestampInner ;
    type Output = CanonicalTimestampSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        CanonicalTimestampSpec::from_structural (input)
    }
}
impl SpecMap for CanonicalTimestampReverse {
    type Input = CanonicalTimestampSpec ;
    type Output = CanonicalTimestampInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

// ============================================================
// Format Specifications
// ============================================================
# [doc = "named format combinator for `date_digit`."]
# [derive (Clone, Copy)]
pub struct DateDigitFmt ;

pub type DateDigitFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl DateDigitFmt {
    # [doc = "specification constructor for `date_digit`."] pub open spec fn spec_inner() -> DateDigitFmtSpec {
        Named ("date_digit",
        Refined (U8,
        | x: u8 | x == 48 || x == 49 || x == 50 || x == 51 || x == 52 || x == 53 || x == 54 || x == 55 || x == 56 || x == 57))
    }
}


# [doc = "named format combinator for `date_t`."]
# [derive (Clone, Copy)]
pub struct DateTFmt ;

pub type DateTFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl DateTFmt {
    # [doc = "specification constructor for `date_t`."] pub open spec fn spec_inner() -> DateTFmtSpec {
        Named ("date_t",
        Refined (U8,
        | x: u8 | x == 84 || x == 116))
    }
}


# [doc = "named format combinator for `date_z`."]
# [derive (Clone, Copy)]
pub struct DateZFmt ;

pub type DateZFmtSpec = Named < Refined < U8, PredFnSpec < u8 >> > ;

impl DateZFmt {
    # [doc = "specification constructor for `date_z`."] pub open spec fn spec_inner() -> DateZFmtSpec {
        Named ("date_z",
        Refined (U8,
        | x: u8 | x == 90 || x == 122))
    }
}


# [doc = "named format combinator for `canonical_timestamp`."]
# [derive (Clone, Copy)]
pub struct CanonicalTimestampFmt ;

pub type CanonicalTimestampFmtSpec = Named < Mapped < Pair < Array < 4, DateDigitFmt >, Pair < Array < 2, DateDigitFmt >, Pair < Array < 2, DateDigitFmt >, Pair < DateTFmt, Pair < Array < 2, DateDigitFmt >, Pair < Array < 2, DateDigitFmt >, Pair < Array < 2, DateDigitFmt >, DateZFmt > > > > > > >, BiMap < CanonicalTimestampForward, CanonicalTimestampReverse >> > ;

impl CanonicalTimestampFmt {
    # [doc = "specification constructor for `canonical_timestamp`."] pub open spec fn spec_inner() -> CanonicalTimestampFmtSpec {
        Named ("canonical_timestamp",
        Mapped {
            inner: Pair (Array::< 4,
            _ > (DateDigitFmt),
            Pair (Array::< 2,
            _ > (DateDigitFmt),
            Pair (Array::< 2,
            _ > (DateDigitFmt),
            Pair (DateTFmt,
            Pair (Array::< 2,
            _ > (DateDigitFmt),
            Pair (Array::< 2,
            _ > (DateDigitFmt),
            Pair (Array::< 2,
            _ > (DateDigitFmt),
            DateZFmt))))))),
            mapper: BiMap (CanonicalTimestampForward,
            CanonicalTimestampReverse),
        }
        )
    }
}

// ============================================================
// Derived Parser, Serializer, Length, and Consistency Specifications
// ============================================================
mod derived_specs {
    use super::*;

    impl SpecParser for DateDigitFmt {
        type PVal = DateDigitSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for DateDigitFmt {
        type Val = DateDigitSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for DateDigitFmt {
        type SValue = DateDigitSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for DateDigitFmt {
        type SVal = DateDigitSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for DateDigitFmt {
        type T = DateDigitSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for DateTFmt {
        type PVal = DateTSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for DateTFmt {
        type Val = DateTSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for DateTFmt {
        type SValue = DateTSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for DateTFmt {
        type SVal = DateTSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for DateTFmt {
        type T = DateTSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for DateZFmt {
        type PVal = DateZSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for DateZFmt {
        type Val = DateZSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for DateZFmt {
        type SValue = DateZSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for DateZFmt {
        type SVal = DateZSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for DateZFmt {
        type T = DateZSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for CanonicalTimestampFmt {
        type PVal = CanonicalTimestampSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for CanonicalTimestampFmt {
        type Val = CanonicalTimestampSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for CanonicalTimestampFmt {
        type SValue = CanonicalTimestampSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for CanonicalTimestampFmt {
        type SVal = CanonicalTimestampSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for CanonicalTimestampFmt {
        type T = CanonicalTimestampSpec ;
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
        CanonicalTimestampSpec::lemma_from_into,
        CanonicalTimestampSpec::lemma_into_from,
    };

    impl SafeParser for DateDigitFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for DateDigitFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for DateDigitFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            reveal(< DateDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            reveal(< DateDigitFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for DateDigitFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for DateDigitFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< DateDigitFmt as SpecSerializer>::spec_serialize) ;
            reveal(< DateDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for DateDigitFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            reveal(< DateDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateDigitFmt as Consistency>::consistent) ;
            reveal(< DateDigitFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for DateDigitFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for DateDigitFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< DateDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateDigitFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for DateDigitFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< DateDigitFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateDigitFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for DateTFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for DateTFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for DateTFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            reveal(< DateTFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            reveal(< DateTFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for DateTFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateTFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for DateTFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< DateTFmt as SpecSerializer>::spec_serialize) ;
            reveal(< DateTFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for DateTFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            reveal(< DateTFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateTFmt as Consistency>::consistent) ;
            reveal(< DateTFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for DateTFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< DateTFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for DateTFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< DateTFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateTFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for DateTFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< DateTFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateTFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for DateZFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for DateZFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for DateZFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            reveal(< DateZFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            reveal(< DateZFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for DateZFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateZFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for DateZFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< DateZFmt as SpecSerializer>::spec_serialize) ;
            reveal(< DateZFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for DateZFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            reveal(< DateZFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateZFmt as Consistency>::consistent) ;
            reveal(< DateZFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for DateZFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< DateZFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for DateZFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< DateZFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateZFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for DateZFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< DateZFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DateZFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for CanonicalTimestampFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for CanonicalTimestampFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for CanonicalTimestampFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            reveal(< CanonicalTimestampFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CanonicalTimestampInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CanonicalTimestampSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            reveal(< CanonicalTimestampFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CanonicalTimestampInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CanonicalTimestampSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for CanonicalTimestampFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CanonicalTimestampFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for CanonicalTimestampFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< CanonicalTimestampFmt as SpecSerializer>::spec_serialize) ;
            reveal(< CanonicalTimestampFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for CanonicalTimestampFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            reveal(< CanonicalTimestampFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CanonicalTimestampFmt as Consistency>::consistent) ;
            reveal(< CanonicalTimestampFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: CanonicalTimestampSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                CanonicalTimestampSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for CanonicalTimestampFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CanonicalTimestampInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CanonicalTimestampSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for CanonicalTimestampFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< CanonicalTimestampFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CanonicalTimestampFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for CanonicalTimestampFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< CanonicalTimestampFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CanonicalTimestampFmt as SpecSerializer>::spec_serialize) ;
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

    impl<'i> Parser<&'i [u8]> for DateDigitFmt {
        type PT = DateDigit;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<DateDigitFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(v == 48 || v == 49 || v == 50 || v == 51 || v == 52 || v == 53 || v == 54 || v == 55 || v == 56 || v == 57) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, DateDigit> for DateDigitFmt {
        fn serialize_into(&self, v: &DateDigit, obuf: &mut Output) {
            reveal(<DateDigitFmt as SpecSerializer>::spec_serialize);
            reveal(<DateDigitFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<DateDigit> for DateDigitFmt {
        fn prepare(&self, v: &DateDigit) -> Result<usize, PreSerializeError> {
            reveal(<DateDigitFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 48 || *v == 49 || *v == 50 || *v == 51 || *v == 52 || *v == 53 || *v == 54 || *v == 55 || *v == 56 || *v == 57) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for DateTFmt {
        type PT = DateT;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<DateTFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(v == 84 || v == 116) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, DateT> for DateTFmt {
        fn serialize_into(&self, v: &DateT, obuf: &mut Output) {
            reveal(<DateTFmt as SpecSerializer>::spec_serialize);
            reveal(<DateTFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<DateT> for DateTFmt {
        fn prepare(&self, v: &DateT) -> Result<usize, PreSerializeError> {
            reveal(<DateTFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 84 || *v == 116) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for DateZFmt {
        type PT = DateZ;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<DateZFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U8.parse(ibuf)?;
            if !(v == 90 || v == 122) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, DateZ> for DateZFmt {
        fn serialize_into(&self, v: &DateZ, obuf: &mut Output) {
            reveal(<DateZFmt as SpecSerializer>::spec_serialize);
            reveal(<DateZFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U8.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<DateZ> for DateZFmt {
        fn prepare(&self, v: &DateZ) -> Result<usize, PreSerializeError> {
            reveal(<DateZFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 90 || *v == 122) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U8).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for CanonicalTimestampFmt {
        type PT = CanonicalTimestamp;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<CanonicalTimestampFmt as SpecParser>::spec_parse);
            reveal(<CanonicalTimestamp as DeepView>::deep_view);
            reveal(CanonicalTimestampSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, year) = (Array::< 4, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, month) = (Array::< 2, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let (n3, day) = (Array::< 2, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n3);
            let (n4, separator) = (Named ("date_t", DateTFmt)).parse (& rest) ?;
            if !(separator == 84 || separator == 116) {
                return Err(ParseError::predicate_failed());
            }
            let rest = rest.skip(n4);
            let (n5, hour) = (Array::< 2, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n5);
            let (n6, minute) = (Array::< 2, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n6);
            let (n7, second) = (Array::< 2, _ > (DateDigitFmt)).parse (& rest) ?;
            let rest = rest.skip(n7);
            let (n8, zone) = (Named ("date_z", DateZFmt)).parse (& rest) ?;
            if !(zone == 90 || zone == 122) {
                return Err(ParseError::predicate_failed());
            }
            let rest = rest.skip(n8);
            let total_n = n1 + n2 + n3 + n4 + n5 + n6 + n7 + n8;
            let final_v = CanonicalTimestamp {
                year,
                month,
                day,
                separator,
                hour,
                minute,
                second,
                zone,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, CanonicalTimestamp> for CanonicalTimestampFmt {
        fn serialize_into(&self, v: &CanonicalTimestamp, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<CanonicalTimestampFmt as SpecSerializer>::spec_serialize);
            reveal(<CanonicalTimestampFmt as SpecByteLen>::byte_len);
            reveal(<CanonicalTimestamp as DeepView>::deep_view);
            reveal(CanonicalTimestampSpec::into_structural);
            let ghost old_obuf = obuf@;

            let CanonicalTimestamp {
                year,
                month,
                day,
                separator,
                hour,
                minute,
                second,
                zone,
            } = v;
            Array::< 4, _ > (DateDigitFmt).serialize_into(year, obuf);
            Array::< 2, _ > (DateDigitFmt).serialize_into(month, obuf);
            Array::< 2, _ > (DateDigitFmt).serialize_into(day, obuf);
            DateTFmt.serialize_into(separator, obuf);
            Array::< 2, _ > (DateDigitFmt).serialize_into(hour, obuf);
            Array::< 2, _ > (DateDigitFmt).serialize_into(minute, obuf);
            Array::< 2, _ > (DateDigitFmt).serialize_into(second, obuf);
            DateZFmt.serialize_into(zone, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<CanonicalTimestamp> for CanonicalTimestampFmt {
        fn prepare(&self, v: &CanonicalTimestamp) -> Result<usize, PreSerializeError> {
            reveal(<CanonicalTimestampFmt as SpecByteLen>::byte_len);
            reveal(<CanonicalTimestamp as DeepView>::deep_view);
            reveal(CanonicalTimestampSpec::into_structural);
            let CanonicalTimestamp {
                year,
                month,
                day,
                separator,
                hour,
                minute,
                second,
                zone,
            } = v;
            let l1 = (Array::< 4, _ > (DateDigitFmt)).prepare (year) ?;
            let l2 = (Array::< 2, _ > (DateDigitFmt)).prepare (month) ?;
            let l3 = (Array::< 2, _ > (DateDigitFmt)).prepare (day) ?;
            let l4 = {
                if ! (* separator == 84 || * separator == 116) {
                    Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                }
                else {
                    (Named ("date_t",
                    DateTFmt)).prepare (separator)
                }
            }
            ?;
            let l5 = (Array::< 2, _ > (DateDigitFmt)).prepare (hour) ?;
            let l6 = (Array::< 2, _ > (DateDigitFmt)).prepare (minute) ?;
            let l7 = (Array::< 2, _ > (DateDigitFmt)).prepare (second) ?;
            let l8 = {
                if ! (* zone == 90 || * zone == 122) {
                    Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
                }
                else {
                    (Named ("date_z",
                    DateZFmt)).prepare (zone)
                }
            }
            ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l3).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l4).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l5).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l6).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l7).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l8).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }

}
}
