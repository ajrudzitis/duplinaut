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
# [doc = "data type for `tar_header`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct TarHeader<'i> {
    pub name: &'i [u8],
    pub mode: &'i [u8],
    pub uid: &'i [u8],
    pub gid: &'i [u8],
    pub size: &'i [u8],
    pub mtime: &'i [u8],
    pub checksum: &'i [u8],
    pub entry_type: u8,
    pub linkname: &'i [u8],
    pub magic_version: &'i [u8],
    pub uname: &'i [u8],
    pub gname: &'i [u8],
    pub device_major: &'i [u8],
    pub device_minor: &'i [u8],
    pub prefix: &'i [u8],
    pub unused: &'i [u8],
}
# [verifier::ext_equal]
pub struct TarHeaderSpec < T0 = Seq < u8 >, T1 = Seq < u8 >, T2 = Seq < u8 >, T3 = Seq < u8 >, T4 = Seq < u8 >, T5 = Seq < u8 >, T6 = Seq < u8 >, T7 = u8, T8 = Seq < u8 >, T9 = Seq < u8 >, T10 = Seq < u8 >, T11 = Seq < u8 >, T12 = Seq < u8 >, T13 = Seq < u8 >, T14 = Seq < u8 >, T15 = Seq < u8 > > {
    pub name: T0,
    pub mode: T1,
    pub uid: T2,
    pub gid: T3,
    pub size: T4,
    pub mtime: T5,
    pub checksum: T6,
    pub entry_type: T7,
    pub linkname: T8,
    pub magic_version: T9,
    pub uname: T10,
    pub gname: T11,
    pub device_major: T12,
    pub device_minor: T13,
    pub prefix: T14,
    pub unused: T15,
}
pub type TarHeaderInner = (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (u8, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, (Seq < u8 >, Seq < u8 >))))))))))))))) ;
impl<'i> DeepView for TarHeader<'i> {
    type V = TarHeaderSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        TarHeaderSpec {
            name: self.name.deep_view(),
            mode: self.mode.deep_view(),
            uid: self.uid.deep_view(),
            gid: self.gid.deep_view(),
            size: self.size.deep_view(),
            mtime: self.mtime.deep_view(),
            checksum: self.checksum.deep_view(),
            entry_type: self.entry_type.deep_view(),
            linkname: self.linkname.deep_view(),
            magic_version: self.magic_version.deep_view(),
            uname: self.uname.deep_view(),
            gname: self.gname.deep_view(),
            device_major: self.device_major.deep_view(),
            device_minor: self.device_minor.deep_view(),
            prefix: self.prefix.deep_view(),
            unused: self.unused.deep_view(),
        }
    }
}
impl<'i> TarHeader<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().name == self.name.deep_view(),
    self.deep_view().mode == self.mode.deep_view(),
    self.deep_view().uid == self.uid.deep_view(),
    self.deep_view().gid == self.gid.deep_view(),
    self.deep_view().size == self.size.deep_view(),
    self.deep_view().mtime == self.mtime.deep_view(),
    self.deep_view().checksum == self.checksum.deep_view(),
    self.deep_view().entry_type == self.entry_type.deep_view(),
    self.deep_view().linkname == self.linkname.deep_view(),
    self.deep_view().magic_version == self.magic_version.deep_view(),
    self.deep_view().uname == self.uname.deep_view(),
    self.deep_view().gname == self.gname.deep_view(),
    self.deep_view().device_major == self.device_major.deep_view(),
    self.deep_view().device_minor == self.device_minor.deep_view(),
    self.deep_view().prefix == self.prefix.deep_view(),
    self.deep_view().unused == self.unused.deep_view(),
    {
        reveal(< TarHeader as DeepView>::deep_view) ;
    }
}
impl < T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15 > TarHeaderSpec < T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    (T7,
    (T8,
    (T9,
    (T10,
    (T11,
    (T12,
    (T13,
    (T14,
    T15)))))))))))))))) -> Self {
        let (name,
        (mode,
        (uid,
        (gid,
        (size,
        (mtime,
        (checksum,
        (entry_type,
        (linkname,
        (magic_version,
        (uname,
        (gname,
        (device_major,
        (device_minor,
        (prefix,
        unused))))))))))))))) = input ;
        Self {
            name,
            mode,
            uid,
            gid,
            size,
            mtime,
            checksum,
            entry_type,
            linkname,
            magic_version,
            uname,
            gname,
            device_major,
            device_minor,
            prefix,
            unused
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    (T7,
    (T8,
    (T9,
    (T10,
    (T11,
    (T12,
    (T13,
    (T14,
    T15))))))))))))))) {
        let Self {
            name,
            mode,
            uid,
            gid,
            size,
            mtime,
            checksum,
            entry_type,
            linkname,
            magic_version,
            uname,
            gname,
            device_major,
            device_minor,
            prefix,
            unused
        }
        = self ;
        (name,
        (mode,
        (uid,
        (gid,
        (size,
        (mtime,
        (checksum,
        (entry_type,
        (linkname,
        (magic_version,
        (uname,
        (gname,
        (device_major,
        (device_minor,
        (prefix,
        unused)))))))))))))))
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(TarHeaderSpec::from_structural) ;
        reveal(TarHeaderSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    (T1,
    (T2,
    (T3,
    (T4,
    (T5,
    (T6,
    (T7,
    (T8,
    (T9,
    (T10,
    (T11,
    (T12,
    (T13,
    (T14,
    T15)))))))))))))))) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(TarHeaderSpec::from_structural) ;
        reveal(TarHeaderSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            name,
            mode,
            uid,
            gid,
            size,
            mtime,
            checksum,
            entry_type,
            linkname,
            magic_version,
            uname,
            gname,
            device_major,
            device_minor,
            prefix,
            unused
        }
        => (name,
        (mode,
        (uid,
        (gid,
        (size,
        (mtime,
        (checksum,
        (entry_type,
        (linkname,
        (magic_version,
        (uname,
        (gname,
        (device_major,
        (device_minor,
        (prefix,
        unused))))))))))))))),
    }
   ,
    {
        reveal(TarHeaderSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct TarHeaderForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct TarHeaderReverse ;
impl SpecMap for TarHeaderForward {
    type Input = TarHeaderInner ;
    type Output = TarHeaderSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        TarHeaderSpec::from_structural (input)
    }
}
impl SpecMap for TarHeaderReverse {
    type Input = TarHeaderSpec ;
    type Output = TarHeaderInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

// ============================================================
// Format Specifications
// ============================================================
# [doc = "named format combinator for `tar_header`."]
# [derive (Clone, Copy)]
pub struct TarHeaderFmt ;

pub type TarHeaderFmtSpec = Named < Mapped < Pair < Fixed < 100 >, Pair < Fixed < 8 >, Pair < Fixed < 8 >, Pair < Fixed < 8 >, Pair < Fixed < 12 >, Pair < Fixed < 12 >, Pair < Fixed < 8 >, Pair < U8, Pair < Fixed < 100 >, Pair < Fixed < 8 >, Pair < Fixed < 32 >, Pair < Fixed < 32 >, Pair < Fixed < 8 >, Pair < Fixed < 8 >, Pair < Fixed < 155 >, Fixed < 12 > > > > > > > > > > > > > > > >, BiMap < TarHeaderForward, TarHeaderReverse >> > ;

impl TarHeaderFmt {
    # [doc = "specification constructor for `tar_header`."] pub open spec fn spec_inner() -> TarHeaderFmtSpec {
        Named ("tar_header",
        Mapped {
            inner: Pair (Fixed::< 100 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 12 >,
            Pair (Fixed::< 12 >,
            Pair (Fixed::< 8 >,
            Pair (U8,
            Pair (Fixed::< 100 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 32 >,
            Pair (Fixed::< 32 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 8 >,
            Pair (Fixed::< 155 >,
            Fixed::< 12 >))))))))))))))),
            mapper: BiMap (TarHeaderForward,
            TarHeaderReverse),
        }
        )
    }
}

// ============================================================
// Derived Parser, Serializer, Length, and Consistency Specifications
// ============================================================
mod derived_specs {
    use super::*;

    impl SpecParser for TarHeaderFmt {
        type PVal = TarHeaderSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for TarHeaderFmt {
        type Val = TarHeaderSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for TarHeaderFmt {
        type SValue = TarHeaderSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for TarHeaderFmt {
        type SVal = TarHeaderSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for TarHeaderFmt {
        type T = TarHeaderSpec ;
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
        TarHeaderSpec::lemma_from_into,
        TarHeaderSpec::lemma_into_from,
    };

    impl SafeParser for TarHeaderFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for TarHeaderFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for TarHeaderFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< TarHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: TarHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                TarHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< TarHeaderFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: TarHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                TarHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for TarHeaderFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< TarHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for TarHeaderFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< TarHeaderFmt as SpecSerializer>::spec_serialize) ;
            reveal(< TarHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for TarHeaderFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            reveal(< TarHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< TarHeaderFmt as Consistency>::consistent) ;
            reveal(< TarHeaderFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: TarHeaderSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                TarHeaderSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for TarHeaderFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: TarHeaderInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                TarHeaderSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for TarHeaderFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< TarHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< TarHeaderFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for TarHeaderFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< TarHeaderFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< TarHeaderFmt as SpecSerializer>::spec_serialize) ;
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

    impl<'i> Parser<&'i [u8]> for TarHeaderFmt {
        type PT = TarHeader<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<TarHeaderFmt as SpecParser>::spec_parse);
            reveal(<TarHeader as DeepView>::deep_view);
            reveal(TarHeaderSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, name) = (Fixed::< 100 >).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, mode) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n2);
            let (n3, uid) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n3);
            let (n4, gid) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n4);
            let (n5, size) = (Fixed::< 12 >).parse (& rest) ?;
            let rest = rest.skip(n5);
            let (n6, mtime) = (Fixed::< 12 >).parse (& rest) ?;
            let rest = rest.skip(n6);
            let (n7, checksum) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n7);
            let (n8, entry_type) = (U8).parse (& rest) ?;
            let rest = rest.skip(n8);
            let (n9, linkname) = (Fixed::< 100 >).parse (& rest) ?;
            let rest = rest.skip(n9);
            let (n10, magic_version) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n10);
            let (n11, uname) = (Fixed::< 32 >).parse (& rest) ?;
            let rest = rest.skip(n11);
            let (n12, gname) = (Fixed::< 32 >).parse (& rest) ?;
            let rest = rest.skip(n12);
            let (n13, device_major) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n13);
            let (n14, device_minor) = (Fixed::< 8 >).parse (& rest) ?;
            let rest = rest.skip(n14);
            let (n15, prefix) = (Fixed::< 155 >).parse (& rest) ?;
            let rest = rest.skip(n15);
            let (n16, unused) = (Fixed::< 12 >).parse (& rest) ?;
            let rest = rest.skip(n16);
            let total_n = n1 + n2 + n3 + n4 + n5 + n6 + n7 + n8 + n9 + n10 + n11 + n12 + n13 + n14 + n15 + n16;
            let final_v = TarHeader {
                name,
                mode,
                uid,
                gid,
                size,
                mtime,
                checksum,
                entry_type,
                linkname,
                magic_version,
                uname,
                gname,
                device_major,
                device_minor,
                prefix,
                unused,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, TarHeader<'i>> for TarHeaderFmt {
        fn serialize_into(&self, v: &TarHeader<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<TarHeaderFmt as SpecSerializer>::spec_serialize);
            reveal(<TarHeaderFmt as SpecByteLen>::byte_len);
            reveal(<TarHeader as DeepView>::deep_view);
            reveal(TarHeaderSpec::into_structural);
            let ghost old_obuf = obuf@;

            let TarHeader {
                name,
                mode,
                uid,
                gid,
                size,
                mtime,
                checksum,
                entry_type,
                linkname,
                magic_version,
                uname,
                gname,
                device_major,
                device_minor,
                prefix,
                unused,
            } = v;
            Fixed::< 100 >.serialize_into(* name, obuf);
            Fixed::< 8 >.serialize_into(* mode, obuf);
            Fixed::< 8 >.serialize_into(* uid, obuf);
            Fixed::< 8 >.serialize_into(* gid, obuf);
            Fixed::< 12 >.serialize_into(* size, obuf);
            Fixed::< 12 >.serialize_into(* mtime, obuf);
            Fixed::< 8 >.serialize_into(* checksum, obuf);
            U8.serialize_into(entry_type, obuf);
            Fixed::< 100 >.serialize_into(* linkname, obuf);
            Fixed::< 8 >.serialize_into(* magic_version, obuf);
            Fixed::< 32 >.serialize_into(* uname, obuf);
            Fixed::< 32 >.serialize_into(* gname, obuf);
            Fixed::< 8 >.serialize_into(* device_major, obuf);
            Fixed::< 8 >.serialize_into(* device_minor, obuf);
            Fixed::< 155 >.serialize_into(* prefix, obuf);
            Fixed::< 12 >.serialize_into(* unused, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<TarHeader<'i>> for TarHeaderFmt {
        fn prepare(&self, v: &TarHeader<'i>) -> Result<usize, PreSerializeError> {
            reveal(<TarHeaderFmt as SpecByteLen>::byte_len);
            reveal(<TarHeader as DeepView>::deep_view);
            reveal(TarHeaderSpec::into_structural);
            let TarHeader {
                name,
                mode,
                uid,
                gid,
                size,
                mtime,
                checksum,
                entry_type,
                linkname,
                magic_version,
                uname,
                gname,
                device_major,
                device_minor,
                prefix,
                unused,
            } = v;
            let l1 = (Fixed::< 100 >).prepare (name) ?;
            let l2 = (Fixed::< 8 >).prepare (mode) ?;
            let l3 = (Fixed::< 8 >).prepare (uid) ?;
            let l4 = (Fixed::< 8 >).prepare (gid) ?;
            let l5 = (Fixed::< 12 >).prepare (size) ?;
            let l6 = (Fixed::< 12 >).prepare (mtime) ?;
            let l7 = (Fixed::< 8 >).prepare (checksum) ?;
            let l8 = (U8).prepare (entry_type) ?;
            let l9 = (Fixed::< 100 >).prepare (linkname) ?;
            let l10 = (Fixed::< 8 >).prepare (magic_version) ?;
            let l11 = (Fixed::< 32 >).prepare (uname) ?;
            let l12 = (Fixed::< 32 >).prepare (gname) ?;
            let l13 = (Fixed::< 8 >).prepare (device_major) ?;
            let l14 = (Fixed::< 8 >).prepare (device_minor) ?;
            let l15 = (Fixed::< 155 >).prepare (prefix) ?;
            let l16 = (Fixed::< 12 >).prepare (unused) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l3).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l4).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l5).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l6).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l7).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l8).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l9).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l10).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l11).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l12).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l13).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l14).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l15).ok_or (PreSerializeError::length_too_large()) ?.checked_add (l16).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }

}
}
