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
# [doc = "data type for `delta_magic`."]
pub type DeltaMagic = u32 ;
pub type DeltaMagicSpec = u32 ;


# [doc = "data type for `literal1`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Literal1<'i> {
    pub length: u8,
    pub data: &'i [u8],
}
# [verifier::ext_equal]
pub struct Literal1Spec < T0 = u8, T1 = Seq < u8 > > {
    pub length: T0,
    pub data: T1,
}
pub type Literal1Inner = (u8, Seq < u8 >) ;
impl<'i> DeepView for Literal1<'i> {
    type V = Literal1Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Literal1Spec {
            length: self.length.deep_view(),
            data: self.data.deep_view(),
        }
    }
}
impl<'i> Literal1<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().length == self.length.deep_view(),
    self.deep_view().data == self.data.deep_view(),
    {
        reveal(< Literal1 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Literal1Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (length,
        data) = input ;
        Self {
            length,
            data
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            length,
            data
        }
        = self ;
        (length,
        data)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Literal1Spec::from_structural) ;
        reveal(Literal1Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Literal1Spec::from_structural) ;
        reveal(Literal1Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            length,
            data
        }
        => (length,
        data),
    }
   ,
    {
        reveal(Literal1Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal1Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal1Reverse ;
impl SpecMap for Literal1Forward {
    type Input = Literal1Inner ;
    type Output = Literal1Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Literal1Spec::from_structural (input)
    }
}
impl SpecMap for Literal1Reverse {
    type Input = Literal1Spec ;
    type Output = Literal1Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `literal2`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Literal2<'i> {
    pub length: u16,
    pub data: &'i [u8],
}
# [verifier::ext_equal]
pub struct Literal2Spec < T0 = u16, T1 = Seq < u8 > > {
    pub length: T0,
    pub data: T1,
}
pub type Literal2Inner = (u16, Seq < u8 >) ;
impl<'i> DeepView for Literal2<'i> {
    type V = Literal2Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Literal2Spec {
            length: self.length.deep_view(),
            data: self.data.deep_view(),
        }
    }
}
impl<'i> Literal2<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().length == self.length.deep_view(),
    self.deep_view().data == self.data.deep_view(),
    {
        reveal(< Literal2 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Literal2Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (length,
        data) = input ;
        Self {
            length,
            data
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            length,
            data
        }
        = self ;
        (length,
        data)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Literal2Spec::from_structural) ;
        reveal(Literal2Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Literal2Spec::from_structural) ;
        reveal(Literal2Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            length,
            data
        }
        => (length,
        data),
    }
   ,
    {
        reveal(Literal2Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal2Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal2Reverse ;
impl SpecMap for Literal2Forward {
    type Input = Literal2Inner ;
    type Output = Literal2Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Literal2Spec::from_structural (input)
    }
}
impl SpecMap for Literal2Reverse {
    type Input = Literal2Spec ;
    type Output = Literal2Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `literal4`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Literal4<'i> {
    pub length: u32,
    pub data: &'i [u8],
}
# [verifier::ext_equal]
pub struct Literal4Spec < T0 = u32, T1 = Seq < u8 > > {
    pub length: T0,
    pub data: T1,
}
pub type Literal4Inner = (u32, Seq < u8 >) ;
impl<'i> DeepView for Literal4<'i> {
    type V = Literal4Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Literal4Spec {
            length: self.length.deep_view(),
            data: self.data.deep_view(),
        }
    }
}
impl<'i> Literal4<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().length == self.length.deep_view(),
    self.deep_view().data == self.data.deep_view(),
    {
        reveal(< Literal4 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Literal4Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (length,
        data) = input ;
        Self {
            length,
            data
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            length,
            data
        }
        = self ;
        (length,
        data)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Literal4Spec::from_structural) ;
        reveal(Literal4Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Literal4Spec::from_structural) ;
        reveal(Literal4Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            length,
            data
        }
        => (length,
        data),
    }
   ,
    {
        reveal(Literal4Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal4Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal4Reverse ;
impl SpecMap for Literal4Forward {
    type Input = Literal4Inner ;
    type Output = Literal4Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Literal4Spec::from_structural (input)
    }
}
impl SpecMap for Literal4Reverse {
    type Input = Literal4Spec ;
    type Output = Literal4Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `literal8`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Literal8<'i> {
    pub length: u64,
    pub data: &'i [u8],
}
# [verifier::ext_equal]
pub struct Literal8Spec < T0 = u64, T1 = Seq < u8 > > {
    pub length: T0,
    pub data: T1,
}
pub type Literal8Inner = (u64, Seq < u8 >) ;
impl<'i> DeepView for Literal8<'i> {
    type V = Literal8Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Literal8Spec {
            length: self.length.deep_view(),
            data: self.data.deep_view(),
        }
    }
}
impl<'i> Literal8<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().length == self.length.deep_view(),
    self.deep_view().data == self.data.deep_view(),
    {
        reveal(< Literal8 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Literal8Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (length,
        data) = input ;
        Self {
            length,
            data
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            length,
            data
        }
        = self ;
        (length,
        data)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Literal8Spec::from_structural) ;
        reveal(Literal8Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Literal8Spec::from_structural) ;
        reveal(Literal8Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            length,
            data
        }
        => (length,
        data),
    }
   ,
    {
        reveal(Literal8Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal8Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Literal8Reverse ;
impl SpecMap for Literal8Forward {
    type Input = Literal8Inner ;
    type Output = Literal8Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Literal8Spec::from_structural (input)
    }
}
impl SpecMap for Literal8Reverse {
    type Input = Literal8Spec ;
    type Output = Literal8Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy1_1`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy11 {
    pub offset: u8,
    pub length: u8,
}
# [verifier::ext_equal]
pub struct Copy11Spec < T0 = u8, T1 = u8 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy11Inner = (u8, u8) ;
impl DeepView for Copy11 {
    type V = Copy11Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy11Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy11 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy11 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy11Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy11Spec::from_structural) ;
        reveal(Copy11Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy11Spec::from_structural) ;
        reveal(Copy11Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy11Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy11Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy11Reverse ;
impl SpecMap for Copy11Forward {
    type Input = Copy11Inner ;
    type Output = Copy11Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy11Spec::from_structural (input)
    }
}
impl SpecMap for Copy11Reverse {
    type Input = Copy11Spec ;
    type Output = Copy11Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy1_2`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy12 {
    pub offset: u8,
    pub length: u16,
}
# [verifier::ext_equal]
pub struct Copy12Spec < T0 = u8, T1 = u16 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy12Inner = (u8, u16) ;
impl DeepView for Copy12 {
    type V = Copy12Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy12Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy12 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy12 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy12Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy12Spec::from_structural) ;
        reveal(Copy12Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy12Spec::from_structural) ;
        reveal(Copy12Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy12Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy12Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy12Reverse ;
impl SpecMap for Copy12Forward {
    type Input = Copy12Inner ;
    type Output = Copy12Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy12Spec::from_structural (input)
    }
}
impl SpecMap for Copy12Reverse {
    type Input = Copy12Spec ;
    type Output = Copy12Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy1_4`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy14 {
    pub offset: u8,
    pub length: u32,
}
# [verifier::ext_equal]
pub struct Copy14Spec < T0 = u8, T1 = u32 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy14Inner = (u8, u32) ;
impl DeepView for Copy14 {
    type V = Copy14Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy14Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy14 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy14 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy14Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy14Spec::from_structural) ;
        reveal(Copy14Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy14Spec::from_structural) ;
        reveal(Copy14Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy14Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy14Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy14Reverse ;
impl SpecMap for Copy14Forward {
    type Input = Copy14Inner ;
    type Output = Copy14Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy14Spec::from_structural (input)
    }
}
impl SpecMap for Copy14Reverse {
    type Input = Copy14Spec ;
    type Output = Copy14Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy1_8`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy18 {
    pub offset: u8,
    pub length: u64,
}
# [verifier::ext_equal]
pub struct Copy18Spec < T0 = u8, T1 = u64 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy18Inner = (u8, u64) ;
impl DeepView for Copy18 {
    type V = Copy18Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy18Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy18 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy18 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy18Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy18Spec::from_structural) ;
        reveal(Copy18Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy18Spec::from_structural) ;
        reveal(Copy18Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy18Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy18Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy18Reverse ;
impl SpecMap for Copy18Forward {
    type Input = Copy18Inner ;
    type Output = Copy18Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy18Spec::from_structural (input)
    }
}
impl SpecMap for Copy18Reverse {
    type Input = Copy18Spec ;
    type Output = Copy18Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy2_1`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy21 {
    pub offset: u16,
    pub length: u8,
}
# [verifier::ext_equal]
pub struct Copy21Spec < T0 = u16, T1 = u8 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy21Inner = (u16, u8) ;
impl DeepView for Copy21 {
    type V = Copy21Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy21Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy21 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy21 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy21Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy21Spec::from_structural) ;
        reveal(Copy21Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy21Spec::from_structural) ;
        reveal(Copy21Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy21Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy21Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy21Reverse ;
impl SpecMap for Copy21Forward {
    type Input = Copy21Inner ;
    type Output = Copy21Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy21Spec::from_structural (input)
    }
}
impl SpecMap for Copy21Reverse {
    type Input = Copy21Spec ;
    type Output = Copy21Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy2_2`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy22 {
    pub offset: u16,
    pub length: u16,
}
# [verifier::ext_equal]
pub struct Copy22Spec < T0 = u16, T1 = u16 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy22Inner = (u16, u16) ;
impl DeepView for Copy22 {
    type V = Copy22Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy22Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy22 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy22 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy22Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy22Spec::from_structural) ;
        reveal(Copy22Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy22Spec::from_structural) ;
        reveal(Copy22Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy22Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy22Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy22Reverse ;
impl SpecMap for Copy22Forward {
    type Input = Copy22Inner ;
    type Output = Copy22Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy22Spec::from_structural (input)
    }
}
impl SpecMap for Copy22Reverse {
    type Input = Copy22Spec ;
    type Output = Copy22Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy2_4`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy24 {
    pub offset: u16,
    pub length: u32,
}
# [verifier::ext_equal]
pub struct Copy24Spec < T0 = u16, T1 = u32 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy24Inner = (u16, u32) ;
impl DeepView for Copy24 {
    type V = Copy24Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy24Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy24 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy24 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy24Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy24Spec::from_structural) ;
        reveal(Copy24Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy24Spec::from_structural) ;
        reveal(Copy24Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy24Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy24Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy24Reverse ;
impl SpecMap for Copy24Forward {
    type Input = Copy24Inner ;
    type Output = Copy24Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy24Spec::from_structural (input)
    }
}
impl SpecMap for Copy24Reverse {
    type Input = Copy24Spec ;
    type Output = Copy24Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy2_8`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy28 {
    pub offset: u16,
    pub length: u64,
}
# [verifier::ext_equal]
pub struct Copy28Spec < T0 = u16, T1 = u64 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy28Inner = (u16, u64) ;
impl DeepView for Copy28 {
    type V = Copy28Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy28Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy28 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy28 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy28Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy28Spec::from_structural) ;
        reveal(Copy28Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy28Spec::from_structural) ;
        reveal(Copy28Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy28Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy28Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy28Reverse ;
impl SpecMap for Copy28Forward {
    type Input = Copy28Inner ;
    type Output = Copy28Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy28Spec::from_structural (input)
    }
}
impl SpecMap for Copy28Reverse {
    type Input = Copy28Spec ;
    type Output = Copy28Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy4_1`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy41 {
    pub offset: u32,
    pub length: u8,
}
# [verifier::ext_equal]
pub struct Copy41Spec < T0 = u32, T1 = u8 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy41Inner = (u32, u8) ;
impl DeepView for Copy41 {
    type V = Copy41Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy41Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy41 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy41 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy41Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy41Spec::from_structural) ;
        reveal(Copy41Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy41Spec::from_structural) ;
        reveal(Copy41Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy41Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy41Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy41Reverse ;
impl SpecMap for Copy41Forward {
    type Input = Copy41Inner ;
    type Output = Copy41Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy41Spec::from_structural (input)
    }
}
impl SpecMap for Copy41Reverse {
    type Input = Copy41Spec ;
    type Output = Copy41Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy4_2`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy42 {
    pub offset: u32,
    pub length: u16,
}
# [verifier::ext_equal]
pub struct Copy42Spec < T0 = u32, T1 = u16 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy42Inner = (u32, u16) ;
impl DeepView for Copy42 {
    type V = Copy42Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy42Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy42 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy42 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy42Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy42Spec::from_structural) ;
        reveal(Copy42Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy42Spec::from_structural) ;
        reveal(Copy42Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy42Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy42Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy42Reverse ;
impl SpecMap for Copy42Forward {
    type Input = Copy42Inner ;
    type Output = Copy42Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy42Spec::from_structural (input)
    }
}
impl SpecMap for Copy42Reverse {
    type Input = Copy42Spec ;
    type Output = Copy42Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy4_4`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy44 {
    pub offset: u32,
    pub length: u32,
}
# [verifier::ext_equal]
pub struct Copy44Spec < T0 = u32, T1 = u32 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy44Inner = (u32, u32) ;
impl DeepView for Copy44 {
    type V = Copy44Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy44Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy44 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy44 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy44Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy44Spec::from_structural) ;
        reveal(Copy44Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy44Spec::from_structural) ;
        reveal(Copy44Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy44Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy44Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy44Reverse ;
impl SpecMap for Copy44Forward {
    type Input = Copy44Inner ;
    type Output = Copy44Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy44Spec::from_structural (input)
    }
}
impl SpecMap for Copy44Reverse {
    type Input = Copy44Spec ;
    type Output = Copy44Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy4_8`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy48 {
    pub offset: u32,
    pub length: u64,
}
# [verifier::ext_equal]
pub struct Copy48Spec < T0 = u32, T1 = u64 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy48Inner = (u32, u64) ;
impl DeepView for Copy48 {
    type V = Copy48Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy48Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy48 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy48 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy48Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy48Spec::from_structural) ;
        reveal(Copy48Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy48Spec::from_structural) ;
        reveal(Copy48Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy48Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy48Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy48Reverse ;
impl SpecMap for Copy48Forward {
    type Input = Copy48Inner ;
    type Output = Copy48Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy48Spec::from_structural (input)
    }
}
impl SpecMap for Copy48Reverse {
    type Input = Copy48Spec ;
    type Output = Copy48Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy8_1`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy81 {
    pub offset: u64,
    pub length: u8,
}
# [verifier::ext_equal]
pub struct Copy81Spec < T0 = u64, T1 = u8 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy81Inner = (u64, u8) ;
impl DeepView for Copy81 {
    type V = Copy81Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy81Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy81 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy81 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy81Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy81Spec::from_structural) ;
        reveal(Copy81Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy81Spec::from_structural) ;
        reveal(Copy81Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy81Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy81Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy81Reverse ;
impl SpecMap for Copy81Forward {
    type Input = Copy81Inner ;
    type Output = Copy81Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy81Spec::from_structural (input)
    }
}
impl SpecMap for Copy81Reverse {
    type Input = Copy81Spec ;
    type Output = Copy81Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy8_2`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy82 {
    pub offset: u64,
    pub length: u16,
}
# [verifier::ext_equal]
pub struct Copy82Spec < T0 = u64, T1 = u16 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy82Inner = (u64, u16) ;
impl DeepView for Copy82 {
    type V = Copy82Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy82Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy82 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy82 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy82Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy82Spec::from_structural) ;
        reveal(Copy82Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy82Spec::from_structural) ;
        reveal(Copy82Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy82Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy82Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy82Reverse ;
impl SpecMap for Copy82Forward {
    type Input = Copy82Inner ;
    type Output = Copy82Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy82Spec::from_structural (input)
    }
}
impl SpecMap for Copy82Reverse {
    type Input = Copy82Spec ;
    type Output = Copy82Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy8_4`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy84 {
    pub offset: u64,
    pub length: u32,
}
# [verifier::ext_equal]
pub struct Copy84Spec < T0 = u64, T1 = u32 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy84Inner = (u64, u32) ;
impl DeepView for Copy84 {
    type V = Copy84Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy84Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy84 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy84 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy84Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy84Spec::from_structural) ;
        reveal(Copy84Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy84Spec::from_structural) ;
        reveal(Copy84Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy84Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy84Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy84Reverse ;
impl SpecMap for Copy84Forward {
    type Input = Copy84Inner ;
    type Output = Copy84Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy84Spec::from_structural (input)
    }
}
impl SpecMap for Copy84Reverse {
    type Input = Copy84Spec ;
    type Output = Copy84Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `copy8_8`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Copy88 {
    pub offset: u64,
    pub length: u64,
}
# [verifier::ext_equal]
pub struct Copy88Spec < T0 = u64, T1 = u64 > {
    pub offset: T0,
    pub length: T1,
}
pub type Copy88Inner = (u64, u64) ;
impl DeepView for Copy88 {
    type V = Copy88Spec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        Copy88Spec {
            offset: self.offset.deep_view(),
            length: self.length.deep_view(),
        }
    }
}
impl Copy88 {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().offset == self.offset.deep_view(),
    self.deep_view().length == self.length.deep_view(),
    {
        reveal(< Copy88 as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > Copy88Spec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (offset,
        length) = input ;
        Self {
            offset,
            length
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            offset,
            length
        }
        = self ;
        (offset,
        length)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(Copy88Spec::from_structural) ;
        reveal(Copy88Spec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(Copy88Spec::from_structural) ;
        reveal(Copy88Spec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            offset,
            length
        }
        => (offset,
        length),
    }
   ,
    {
        reveal(Copy88Spec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy88Forward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct Copy88Reverse ;
impl SpecMap for Copy88Forward {
    type Input = Copy88Inner ;
    type Output = Copy88Spec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        Copy88Spec::from_structural (input)
    }
}
impl SpecMap for Copy88Reverse {
    type Input = Copy88Spec ;
    type Output = Copy88Inner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `command`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub struct Command<'i> {
    pub opcode: u8,
    pub body: CommandBody<'i>,
}
# [verifier::ext_equal]
pub struct CommandSpec < T0 = u8, T1 = CommandBodySpec > {
    pub opcode: T0,
    pub body: T1,
}
pub type CommandInner = (u8, CommandBodySpec) ;
impl<'i> DeepView for Command<'i> {
    type V = CommandSpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        CommandSpec {
            opcode: self.opcode.deep_view(),
            body: self.body.deep_view(),
        }
    }
}
impl<'i> Command<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view().opcode == self.opcode.deep_view(),
    self.deep_view().body == self.body.deep_view(),
    {
        reveal(< Command as DeepView>::deep_view) ;
    }
}
impl < T0, T1 > CommandSpec < T0, T1 > {
    # [verifier::opaque] pub open spec fn from_structural (input: (T0,
    T1)) -> Self {
        let (opcode,
        body) = input ;
        Self {
            opcode,
            body
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> (T0,
    T1) {
        let Self {
            opcode,
            body
        }
        = self ;
        (opcode,
        body)
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(CommandSpec::from_structural) ;
        reveal(CommandSpec::into_structural) ;
    }
    pub broadcast proof fn lemma_into_from (input: (T0,
    T1)) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(CommandSpec::from_structural) ;
        reveal(CommandSpec::into_structural) ;
    }
    pub proof fn lemma_into_structural_fields (self) ensures Self::into_structural (self) == match self {
        Self {
            opcode,
            body
        }
        => (opcode,
        body),
    }
   ,
    {
        reveal(CommandSpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CommandForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CommandReverse ;
impl SpecMap for CommandForward {
    type Input = CommandInner ;
    type Output = CommandSpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        CommandSpec::from_structural (input)
    }
}
impl SpecMap for CommandReverse {
    type Input = CommandSpec ;
    type Output = CommandInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

# [doc = "data type for `command_body`."]
# [derive (Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommandBody<'i> {
    Variant1 (()),
    Variant2 (&'i [u8]),
    Variant3 (Literal1<'i>),
    Variant4 (Literal2<'i>),
    Variant5 (Literal4<'i>),
    Variant6 (Literal8<'i>),
    Variant7 (Copy11),
    Variant8 (Copy12),
    Variant9 (Copy14),
    Variant10 (Copy18),
    Variant11 (Copy21),
    Variant12 (Copy22),
    Variant13 (Copy24),
    Variant14 (Copy28),
    Variant15 (Copy41),
    Variant16 (Copy42),
    Variant17 (Copy44),
    Variant18 (Copy48),
    Variant19 (Copy81),
    Variant20 (Copy82),
    Variant21 (Copy84),
    Variant22 (Copy88),
    Default (Never),
}
# [verifier::ext_equal]
pub enum CommandBodySpec < T0 =(), T1 = Seq < u8 >, T2 = Literal1Spec, T3 = Literal2Spec, T4 = Literal4Spec, T5 = Literal8Spec, T6 = Copy11Spec, T7 = Copy12Spec, T8 = Copy14Spec, T9 = Copy18Spec, T10 = Copy21Spec, T11 = Copy22Spec, T12 = Copy24Spec, T13 = Copy28Spec, T14 = Copy41Spec, T15 = Copy42Spec, T16 = Copy44Spec, T17 = Copy48Spec, T18 = Copy81Spec, T19 = Copy82Spec, T20 = Copy84Spec, T21 = Copy88Spec, T22 = Never > {
    Variant1 (T0),
    Variant2 (T1),
    Variant3 (T2),
    Variant4 (T3),
    Variant5 (T4),
    Variant6 (T5),
    Variant7 (T6),
    Variant8 (T7),
    Variant9 (T8),
    Variant10 (T9),
    Variant11 (T10),
    Variant12 (T11),
    Variant13 (T12),
    Variant14 (T13),
    Variant15 (T14),
    Variant16 (T15),
    Variant17 (T16),
    Variant18 (T17),
    Variant19 (T18),
    Variant20 (T19),
    Variant21 (T20),
    Variant22 (T21),
    Default (T22),
}
pub type CommandBodyInner = Sum < Sum < Sum < Sum <(), Seq < u8 > >, Sum < Literal1Spec, Sum < Literal2Spec, Literal4Spec > > >, Sum < Sum < Literal8Spec, Sum < Copy11Spec, Copy12Spec > >, Sum < Copy14Spec, Sum < Copy18Spec, Copy21Spec > > > >, Sum < Sum < Sum < Copy22Spec, Sum < Copy24Spec, Copy28Spec > >, Sum < Copy41Spec, Sum < Copy42Spec, Copy44Spec > > >, Sum < Sum < Copy48Spec, Sum < Copy81Spec, Copy82Spec > >, Sum < Copy84Spec, Sum < Copy88Spec, Never > > > > > ;
impl<'i> DeepView for CommandBody<'i> {
    type V = CommandBodySpec ;
    # [verifier::opaque] open spec fn deep_view (& self) -> Self::V {
        match self {
            CommandBody::Variant1 (v) => CommandBodySpec::Variant1 (v.deep_view()),
            CommandBody::Variant2 (v) => CommandBodySpec::Variant2 (v.deep_view()),
            CommandBody::Variant3 (v) => CommandBodySpec::Variant3 (v.deep_view()),
            CommandBody::Variant4 (v) => CommandBodySpec::Variant4 (v.deep_view()),
            CommandBody::Variant5 (v) => CommandBodySpec::Variant5 (v.deep_view()),
            CommandBody::Variant6 (v) => CommandBodySpec::Variant6 (v.deep_view()),
            CommandBody::Variant7 (v) => CommandBodySpec::Variant7 (v.deep_view()),
            CommandBody::Variant8 (v) => CommandBodySpec::Variant8 (v.deep_view()),
            CommandBody::Variant9 (v) => CommandBodySpec::Variant9 (v.deep_view()),
            CommandBody::Variant10 (v) => CommandBodySpec::Variant10 (v.deep_view()),
            CommandBody::Variant11 (v) => CommandBodySpec::Variant11 (v.deep_view()),
            CommandBody::Variant12 (v) => CommandBodySpec::Variant12 (v.deep_view()),
            CommandBody::Variant13 (v) => CommandBodySpec::Variant13 (v.deep_view()),
            CommandBody::Variant14 (v) => CommandBodySpec::Variant14 (v.deep_view()),
            CommandBody::Variant15 (v) => CommandBodySpec::Variant15 (v.deep_view()),
            CommandBody::Variant16 (v) => CommandBodySpec::Variant16 (v.deep_view()),
            CommandBody::Variant17 (v) => CommandBodySpec::Variant17 (v.deep_view()),
            CommandBody::Variant18 (v) => CommandBodySpec::Variant18 (v.deep_view()),
            CommandBody::Variant19 (v) => CommandBodySpec::Variant19 (v.deep_view()),
            CommandBody::Variant20 (v) => CommandBodySpec::Variant20 (v.deep_view()),
            CommandBody::Variant21 (v) => CommandBodySpec::Variant21 (v.deep_view()),
            CommandBody::Variant22 (v) => CommandBodySpec::Variant22 (v.deep_view()),
            CommandBody::Default (v) => CommandBodySpec::Default (v.deep_view()),
        }
    }
}
impl<'i> CommandBody<'i> {
    pub proof fn lemma_deep_view_fields (& self) ensures self.deep_view() == match self {
        CommandBody::Variant1 (v) => CommandBodySpec::Variant1 (v.deep_view()),
        CommandBody::Variant2 (v) => CommandBodySpec::Variant2 (v.deep_view()),
        CommandBody::Variant3 (v) => CommandBodySpec::Variant3 (v.deep_view()),
        CommandBody::Variant4 (v) => CommandBodySpec::Variant4 (v.deep_view()),
        CommandBody::Variant5 (v) => CommandBodySpec::Variant5 (v.deep_view()),
        CommandBody::Variant6 (v) => CommandBodySpec::Variant6 (v.deep_view()),
        CommandBody::Variant7 (v) => CommandBodySpec::Variant7 (v.deep_view()),
        CommandBody::Variant8 (v) => CommandBodySpec::Variant8 (v.deep_view()),
        CommandBody::Variant9 (v) => CommandBodySpec::Variant9 (v.deep_view()),
        CommandBody::Variant10 (v) => CommandBodySpec::Variant10 (v.deep_view()),
        CommandBody::Variant11 (v) => CommandBodySpec::Variant11 (v.deep_view()),
        CommandBody::Variant12 (v) => CommandBodySpec::Variant12 (v.deep_view()),
        CommandBody::Variant13 (v) => CommandBodySpec::Variant13 (v.deep_view()),
        CommandBody::Variant14 (v) => CommandBodySpec::Variant14 (v.deep_view()),
        CommandBody::Variant15 (v) => CommandBodySpec::Variant15 (v.deep_view()),
        CommandBody::Variant16 (v) => CommandBodySpec::Variant16 (v.deep_view()),
        CommandBody::Variant17 (v) => CommandBodySpec::Variant17 (v.deep_view()),
        CommandBody::Variant18 (v) => CommandBodySpec::Variant18 (v.deep_view()),
        CommandBody::Variant19 (v) => CommandBodySpec::Variant19 (v.deep_view()),
        CommandBody::Variant20 (v) => CommandBodySpec::Variant20 (v.deep_view()),
        CommandBody::Variant21 (v) => CommandBodySpec::Variant21 (v.deep_view()),
        CommandBody::Variant22 (v) => CommandBodySpec::Variant22 (v.deep_view()),
        CommandBody::Default (v) => CommandBodySpec::Default (v.deep_view()),
    }
   ,
    {
        reveal(< CommandBody as DeepView>::deep_view) ;
    }
}
impl < T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22 > CommandBodySpec < T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22 > {
    # [verifier::opaque] pub open spec fn from_structural (input: Sum < Sum < Sum < Sum < T0,
    T1 >,
    Sum < T2,
    Sum < T3,
    T4 > > >,
    Sum < Sum < T5,
    Sum < T6,
    T7 > >,
    Sum < T8,
    Sum < T9,
    T10 > > > >,
    Sum < Sum < Sum < T11,
    Sum < T12,
    T13 > >,
    Sum < T14,
    Sum < T15,
    T16 > > >,
    Sum < Sum < T17,
    Sum < T18,
    T19 > >,
    Sum < T20,
    Sum < T21,
    T22 > > > > >) -> Self {
        match input {
            L (L (L (L (value)))) => Self::Variant1 (value),
            L (L (L (R (value)))) => Self::Variant2 (value),
            L (L (R (L (value)))) => Self::Variant3 (value),
            L (L (R (R (L (value))))) => Self::Variant4 (value),
            L (L (R (R (R (value))))) => Self::Variant5 (value),
            L (R (L (L (value)))) => Self::Variant6 (value),
            L (R (L (R (L (value))))) => Self::Variant7 (value),
            L (R (L (R (R (value))))) => Self::Variant8 (value),
            L (R (R (L (value)))) => Self::Variant9 (value),
            L (R (R (R (L (value))))) => Self::Variant10 (value),
            L (R (R (R (R (value))))) => Self::Variant11 (value),
            R (L (L (L (value)))) => Self::Variant12 (value),
            R (L (L (R (L (value))))) => Self::Variant13 (value),
            R (L (L (R (R (value))))) => Self::Variant14 (value),
            R (L (R (L (value)))) => Self::Variant15 (value),
            R (L (R (R (L (value))))) => Self::Variant16 (value),
            R (L (R (R (R (value))))) => Self::Variant17 (value),
            R (R (L (L (value)))) => Self::Variant18 (value),
            R (R (L (R (L (value))))) => Self::Variant19 (value),
            R (R (L (R (R (value))))) => Self::Variant20 (value),
            R (R (R (L (value)))) => Self::Variant21 (value),
            R (R (R (R (L (value))))) => Self::Variant22 (value),
            R (R (R (R (R (value))))) => Self::Default (value),
        }
    }
    # [verifier::opaque] pub open spec fn into_structural (self) -> Sum < Sum < Sum < Sum < T0,
    T1 >,
    Sum < T2,
    Sum < T3,
    T4 > > >,
    Sum < Sum < T5,
    Sum < T6,
    T7 > >,
    Sum < T8,
    Sum < T9,
    T10 > > > >,
    Sum < Sum < Sum < T11,
    Sum < T12,
    T13 > >,
    Sum < T14,
    Sum < T15,
    T16 > > >,
    Sum < Sum < T17,
    Sum < T18,
    T19 > >,
    Sum < T20,
    Sum < T21,
    T22 > > > > > {
        match self {
            Self::Variant1 (value) => L (L (L (L (value)))),
            Self::Variant2 (value) => L (L (L (R (value)))),
            Self::Variant3 (value) => L (L (R (L (value)))),
            Self::Variant4 (value) => L (L (R (R (L (value))))),
            Self::Variant5 (value) => L (L (R (R (R (value))))),
            Self::Variant6 (value) => L (R (L (L (value)))),
            Self::Variant7 (value) => L (R (L (R (L (value))))),
            Self::Variant8 (value) => L (R (L (R (R (value))))),
            Self::Variant9 (value) => L (R (R (L (value)))),
            Self::Variant10 (value) => L (R (R (R (L (value))))),
            Self::Variant11 (value) => L (R (R (R (R (value))))),
            Self::Variant12 (value) => R (L (L (L (value)))),
            Self::Variant13 (value) => R (L (L (R (L (value))))),
            Self::Variant14 (value) => R (L (L (R (R (value))))),
            Self::Variant15 (value) => R (L (R (L (value)))),
            Self::Variant16 (value) => R (L (R (R (L (value))))),
            Self::Variant17 (value) => R (L (R (R (R (value))))),
            Self::Variant18 (value) => R (R (L (L (value)))),
            Self::Variant19 (value) => R (R (L (R (L (value))))),
            Self::Variant20 (value) => R (R (L (R (R (value))))),
            Self::Variant21 (value) => R (R (R (L (value)))),
            Self::Variant22 (value) => R (R (R (R (L (value))))),
            Self::Default (value) => R (R (R (R (R (value))))),
        }
    }
    pub broadcast proof fn lemma_from_into (self) ensures # [trigger] Self::from_structural (Self::into_structural (self)) == self,
    {
        reveal(CommandBodySpec::from_structural) ;
        reveal(CommandBodySpec::into_structural) ;
        match self {
            Self::Variant1 (_) => {
            }
           ,
            Self::Variant2 (_) => {
            }
           ,
            Self::Variant3 (_) => {
            }
           ,
            Self::Variant4 (_) => {
            }
           ,
            Self::Variant5 (_) => {
            }
           ,
            Self::Variant6 (_) => {
            }
           ,
            Self::Variant7 (_) => {
            }
           ,
            Self::Variant8 (_) => {
            }
           ,
            Self::Variant9 (_) => {
            }
           ,
            Self::Variant10 (_) => {
            }
           ,
            Self::Variant11 (_) => {
            }
           ,
            Self::Variant12 (_) => {
            }
           ,
            Self::Variant13 (_) => {
            }
           ,
            Self::Variant14 (_) => {
            }
           ,
            Self::Variant15 (_) => {
            }
           ,
            Self::Variant16 (_) => {
            }
           ,
            Self::Variant17 (_) => {
            }
           ,
            Self::Variant18 (_) => {
            }
           ,
            Self::Variant19 (_) => {
            }
           ,
            Self::Variant20 (_) => {
            }
           ,
            Self::Variant21 (_) => {
            }
           ,
            Self::Variant22 (_) => {
            }
           ,
            Self::Default (_) => {
            }
           ,
        }
    }
    pub broadcast proof fn lemma_into_from (input: Sum < Sum < Sum < Sum < T0,
    T1 >,
    Sum < T2,
    Sum < T3,
    T4 > > >,
    Sum < Sum < T5,
    Sum < T6,
    T7 > >,
    Sum < T8,
    Sum < T9,
    T10 > > > >,
    Sum < Sum < Sum < T11,
    Sum < T12,
    T13 > >,
    Sum < T14,
    Sum < T15,
    T16 > > >,
    Sum < Sum < T17,
    Sum < T18,
    T19 > >,
    Sum < T20,
    Sum < T21,
    T22 > > > > >) ensures # [trigger] Self::into_structural (Self::from_structural (input)) == input,
    {
        reveal(CommandBodySpec::from_structural) ;
        reveal(CommandBodySpec::into_structural) ;
        match input {
            L (L (L (L (_)))) => {
            }
           ,
            L (L (L (R (_)))) => {
            }
           ,
            L (L (R (L (_)))) => {
            }
           ,
            L (L (R (R (L (_))))) => {
            }
           ,
            L (L (R (R (R (_))))) => {
            }
           ,
            L (R (L (L (_)))) => {
            }
           ,
            L (R (L (R (L (_))))) => {
            }
           ,
            L (R (L (R (R (_))))) => {
            }
           ,
            L (R (R (L (_)))) => {
            }
           ,
            L (R (R (R (L (_))))) => {
            }
           ,
            L (R (R (R (R (_))))) => {
            }
           ,
            R (L (L (L (_)))) => {
            }
           ,
            R (L (L (R (L (_))))) => {
            }
           ,
            R (L (L (R (R (_))))) => {
            }
           ,
            R (L (R (L (_)))) => {
            }
           ,
            R (L (R (R (L (_))))) => {
            }
           ,
            R (L (R (R (R (_))))) => {
            }
           ,
            R (R (L (L (_)))) => {
            }
           ,
            R (R (L (R (L (_))))) => {
            }
           ,
            R (R (L (R (R (_))))) => {
            }
           ,
            R (R (R (L (_)))) => {
            }
           ,
            R (R (R (R (L (_))))) => {
            }
           ,
            R (R (R (R (R (_))))) => {
            }
           ,
        }
    }
    pub proof fn lemma_into_structural_variant (self) ensures Self::into_structural (self) == match self {
        Self::Variant1 (value) => L (L (L (L (value)))),
        Self::Variant2 (value) => L (L (L (R (value)))),
        Self::Variant3 (value) => L (L (R (L (value)))),
        Self::Variant4 (value) => L (L (R (R (L (value))))),
        Self::Variant5 (value) => L (L (R (R (R (value))))),
        Self::Variant6 (value) => L (R (L (L (value)))),
        Self::Variant7 (value) => L (R (L (R (L (value))))),
        Self::Variant8 (value) => L (R (L (R (R (value))))),
        Self::Variant9 (value) => L (R (R (L (value)))),
        Self::Variant10 (value) => L (R (R (R (L (value))))),
        Self::Variant11 (value) => L (R (R (R (R (value))))),
        Self::Variant12 (value) => R (L (L (L (value)))),
        Self::Variant13 (value) => R (L (L (R (L (value))))),
        Self::Variant14 (value) => R (L (L (R (R (value))))),
        Self::Variant15 (value) => R (L (R (L (value)))),
        Self::Variant16 (value) => R (L (R (R (L (value))))),
        Self::Variant17 (value) => R (L (R (R (R (value))))),
        Self::Variant18 (value) => R (R (L (L (value)))),
        Self::Variant19 (value) => R (R (L (R (L (value))))),
        Self::Variant20 (value) => R (R (L (R (R (value))))),
        Self::Variant21 (value) => R (R (R (L (value)))),
        Self::Variant22 (value) => R (R (R (R (L (value))))),
        Self::Default (value) => R (R (R (R (R (value))))),
    }
   ,
    {
        reveal(CommandBodySpec::into_structural) ;
    }
}
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CommandBodyForward ;
# [derive (Clone, Copy)]
# [doc (hidden)]
pub struct CommandBodyReverse ;
impl SpecMap for CommandBodyForward {
    type Input = CommandBodyInner ;
    type Output = CommandBodySpec ;
    open spec fn spec_map (& self,
    input: Self::Input) -> Self::Output {
        CommandBodySpec::from_structural (input)
    }
}
impl SpecMap for CommandBodyReverse {
    type Input = CommandBodySpec ;
    type Output = CommandBodyInner ;
    open spec fn spec_map (& self,
    value: Self::Input) -> Self::Output {
        value.into_structural()
    }
}

// ============================================================
// Format Specifications
// ============================================================
# [doc = "named format combinator for `delta_magic`."]
# [derive (Clone, Copy)]
pub struct DeltaMagicFmt ;

pub type DeltaMagicFmtSpec = Named < Refined < U32Be, PredFnSpec < u32 >> > ;

impl DeltaMagicFmt {
    # [doc = "specification constructor for `delta_magic`."] pub open spec fn spec_inner() -> DeltaMagicFmtSpec {
        Named ("delta_magic",
        Refined (U32Be,
        | x: u32 | x == 1920139830))
    }
}


# [doc = "named format combinator for `literal1`."]
# [derive (Clone, Copy)]
pub struct Literal1Fmt ;

pub type Literal1FmtSpec = Named < Mapped < Bind < U8, spec_fn (u8) -> Varied < u8 > >, BiMap < Literal1Forward, Literal1Reverse >> > ;

impl Literal1Fmt {
    # [doc = "specification constructor for `literal1`."] pub open spec fn spec_inner() -> Literal1FmtSpec {
        Named ("literal1",
        Mapped {
            inner: Bind (U8,
            | length: u8 | Varied (length)),
            mapper: BiMap (Literal1Forward,
            Literal1Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `literal2`."]
# [derive (Clone, Copy)]
pub struct Literal2Fmt ;

pub type Literal2FmtSpec = Named < Mapped < Bind < U16Be, spec_fn (u16) -> Varied < u16 > >, BiMap < Literal2Forward, Literal2Reverse >> > ;

impl Literal2Fmt {
    # [doc = "specification constructor for `literal2`."] pub open spec fn spec_inner() -> Literal2FmtSpec {
        Named ("literal2",
        Mapped {
            inner: Bind (U16Be,
            | length: u16 | Varied (length)),
            mapper: BiMap (Literal2Forward,
            Literal2Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `literal4`."]
# [derive (Clone, Copy)]
pub struct Literal4Fmt ;

pub type Literal4FmtSpec = Named < Mapped < Bind < U32Be, spec_fn (u32) -> Varied < u32 > >, BiMap < Literal4Forward, Literal4Reverse >> > ;

impl Literal4Fmt {
    # [doc = "specification constructor for `literal4`."] pub open spec fn spec_inner() -> Literal4FmtSpec {
        Named ("literal4",
        Mapped {
            inner: Bind (U32Be,
            | length: u32 | Varied (length)),
            mapper: BiMap (Literal4Forward,
            Literal4Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `literal8`."]
# [derive (Clone, Copy)]
pub struct Literal8Fmt ;

pub type Literal8FmtSpec = Named < Mapped < Bind < U64Be, spec_fn (u64) -> Varied < u64 > >, BiMap < Literal8Forward, Literal8Reverse >> > ;

impl Literal8Fmt {
    # [doc = "specification constructor for `literal8`."] pub open spec fn spec_inner() -> Literal8FmtSpec {
        Named ("literal8",
        Mapped {
            inner: Bind (U64Be,
            | length: u64 | Varied (length)),
            mapper: BiMap (Literal8Forward,
            Literal8Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy1_1`."]
# [derive (Clone, Copy)]
pub struct Copy11Fmt ;

pub type Copy11FmtSpec = Named < Mapped < Pair < U8, U8 >, BiMap < Copy11Forward, Copy11Reverse >> > ;

impl Copy11Fmt {
    # [doc = "specification constructor for `copy1_1`."] pub open spec fn spec_inner() -> Copy11FmtSpec {
        Named ("copy1_1",
        Mapped {
            inner: Pair (U8,
            U8),
            mapper: BiMap (Copy11Forward,
            Copy11Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy1_2`."]
# [derive (Clone, Copy)]
pub struct Copy12Fmt ;

pub type Copy12FmtSpec = Named < Mapped < Pair < U8, U16Be >, BiMap < Copy12Forward, Copy12Reverse >> > ;

impl Copy12Fmt {
    # [doc = "specification constructor for `copy1_2`."] pub open spec fn spec_inner() -> Copy12FmtSpec {
        Named ("copy1_2",
        Mapped {
            inner: Pair (U8,
            U16Be),
            mapper: BiMap (Copy12Forward,
            Copy12Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy1_4`."]
# [derive (Clone, Copy)]
pub struct Copy14Fmt ;

pub type Copy14FmtSpec = Named < Mapped < Pair < U8, U32Be >, BiMap < Copy14Forward, Copy14Reverse >> > ;

impl Copy14Fmt {
    # [doc = "specification constructor for `copy1_4`."] pub open spec fn spec_inner() -> Copy14FmtSpec {
        Named ("copy1_4",
        Mapped {
            inner: Pair (U8,
            U32Be),
            mapper: BiMap (Copy14Forward,
            Copy14Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy1_8`."]
# [derive (Clone, Copy)]
pub struct Copy18Fmt ;

pub type Copy18FmtSpec = Named < Mapped < Pair < U8, U64Be >, BiMap < Copy18Forward, Copy18Reverse >> > ;

impl Copy18Fmt {
    # [doc = "specification constructor for `copy1_8`."] pub open spec fn spec_inner() -> Copy18FmtSpec {
        Named ("copy1_8",
        Mapped {
            inner: Pair (U8,
            U64Be),
            mapper: BiMap (Copy18Forward,
            Copy18Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy2_1`."]
# [derive (Clone, Copy)]
pub struct Copy21Fmt ;

pub type Copy21FmtSpec = Named < Mapped < Pair < U16Be, U8 >, BiMap < Copy21Forward, Copy21Reverse >> > ;

impl Copy21Fmt {
    # [doc = "specification constructor for `copy2_1`."] pub open spec fn spec_inner() -> Copy21FmtSpec {
        Named ("copy2_1",
        Mapped {
            inner: Pair (U16Be,
            U8),
            mapper: BiMap (Copy21Forward,
            Copy21Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy2_2`."]
# [derive (Clone, Copy)]
pub struct Copy22Fmt ;

pub type Copy22FmtSpec = Named < Mapped < Pair < U16Be, U16Be >, BiMap < Copy22Forward, Copy22Reverse >> > ;

impl Copy22Fmt {
    # [doc = "specification constructor for `copy2_2`."] pub open spec fn spec_inner() -> Copy22FmtSpec {
        Named ("copy2_2",
        Mapped {
            inner: Pair (U16Be,
            U16Be),
            mapper: BiMap (Copy22Forward,
            Copy22Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy2_4`."]
# [derive (Clone, Copy)]
pub struct Copy24Fmt ;

pub type Copy24FmtSpec = Named < Mapped < Pair < U16Be, U32Be >, BiMap < Copy24Forward, Copy24Reverse >> > ;

impl Copy24Fmt {
    # [doc = "specification constructor for `copy2_4`."] pub open spec fn spec_inner() -> Copy24FmtSpec {
        Named ("copy2_4",
        Mapped {
            inner: Pair (U16Be,
            U32Be),
            mapper: BiMap (Copy24Forward,
            Copy24Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy2_8`."]
# [derive (Clone, Copy)]
pub struct Copy28Fmt ;

pub type Copy28FmtSpec = Named < Mapped < Pair < U16Be, U64Be >, BiMap < Copy28Forward, Copy28Reverse >> > ;

impl Copy28Fmt {
    # [doc = "specification constructor for `copy2_8`."] pub open spec fn spec_inner() -> Copy28FmtSpec {
        Named ("copy2_8",
        Mapped {
            inner: Pair (U16Be,
            U64Be),
            mapper: BiMap (Copy28Forward,
            Copy28Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy4_1`."]
# [derive (Clone, Copy)]
pub struct Copy41Fmt ;

pub type Copy41FmtSpec = Named < Mapped < Pair < U32Be, U8 >, BiMap < Copy41Forward, Copy41Reverse >> > ;

impl Copy41Fmt {
    # [doc = "specification constructor for `copy4_1`."] pub open spec fn spec_inner() -> Copy41FmtSpec {
        Named ("copy4_1",
        Mapped {
            inner: Pair (U32Be,
            U8),
            mapper: BiMap (Copy41Forward,
            Copy41Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy4_2`."]
# [derive (Clone, Copy)]
pub struct Copy42Fmt ;

pub type Copy42FmtSpec = Named < Mapped < Pair < U32Be, U16Be >, BiMap < Copy42Forward, Copy42Reverse >> > ;

impl Copy42Fmt {
    # [doc = "specification constructor for `copy4_2`."] pub open spec fn spec_inner() -> Copy42FmtSpec {
        Named ("copy4_2",
        Mapped {
            inner: Pair (U32Be,
            U16Be),
            mapper: BiMap (Copy42Forward,
            Copy42Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy4_4`."]
# [derive (Clone, Copy)]
pub struct Copy44Fmt ;

pub type Copy44FmtSpec = Named < Mapped < Pair < U32Be, U32Be >, BiMap < Copy44Forward, Copy44Reverse >> > ;

impl Copy44Fmt {
    # [doc = "specification constructor for `copy4_4`."] pub open spec fn spec_inner() -> Copy44FmtSpec {
        Named ("copy4_4",
        Mapped {
            inner: Pair (U32Be,
            U32Be),
            mapper: BiMap (Copy44Forward,
            Copy44Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy4_8`."]
# [derive (Clone, Copy)]
pub struct Copy48Fmt ;

pub type Copy48FmtSpec = Named < Mapped < Pair < U32Be, U64Be >, BiMap < Copy48Forward, Copy48Reverse >> > ;

impl Copy48Fmt {
    # [doc = "specification constructor for `copy4_8`."] pub open spec fn spec_inner() -> Copy48FmtSpec {
        Named ("copy4_8",
        Mapped {
            inner: Pair (U32Be,
            U64Be),
            mapper: BiMap (Copy48Forward,
            Copy48Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy8_1`."]
# [derive (Clone, Copy)]
pub struct Copy81Fmt ;

pub type Copy81FmtSpec = Named < Mapped < Pair < U64Be, U8 >, BiMap < Copy81Forward, Copy81Reverse >> > ;

impl Copy81Fmt {
    # [doc = "specification constructor for `copy8_1`."] pub open spec fn spec_inner() -> Copy81FmtSpec {
        Named ("copy8_1",
        Mapped {
            inner: Pair (U64Be,
            U8),
            mapper: BiMap (Copy81Forward,
            Copy81Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy8_2`."]
# [derive (Clone, Copy)]
pub struct Copy82Fmt ;

pub type Copy82FmtSpec = Named < Mapped < Pair < U64Be, U16Be >, BiMap < Copy82Forward, Copy82Reverse >> > ;

impl Copy82Fmt {
    # [doc = "specification constructor for `copy8_2`."] pub open spec fn spec_inner() -> Copy82FmtSpec {
        Named ("copy8_2",
        Mapped {
            inner: Pair (U64Be,
            U16Be),
            mapper: BiMap (Copy82Forward,
            Copy82Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy8_4`."]
# [derive (Clone, Copy)]
pub struct Copy84Fmt ;

pub type Copy84FmtSpec = Named < Mapped < Pair < U64Be, U32Be >, BiMap < Copy84Forward, Copy84Reverse >> > ;

impl Copy84Fmt {
    # [doc = "specification constructor for `copy8_4`."] pub open spec fn spec_inner() -> Copy84FmtSpec {
        Named ("copy8_4",
        Mapped {
            inner: Pair (U64Be,
            U32Be),
            mapper: BiMap (Copy84Forward,
            Copy84Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `copy8_8`."]
# [derive (Clone, Copy)]
pub struct Copy88Fmt ;

pub type Copy88FmtSpec = Named < Mapped < Pair < U64Be, U64Be >, BiMap < Copy88Forward, Copy88Reverse >> > ;

impl Copy88Fmt {
    # [doc = "specification constructor for `copy8_8`."] pub open spec fn spec_inner() -> Copy88FmtSpec {
        Named ("copy8_8",
        Mapped {
            inner: Pair (U64Be,
            U64Be),
            mapper: BiMap (Copy88Forward,
            Copy88Reverse),
        }
        )
    }
}


# [doc = "named format combinator for `command`."]
# [derive (Clone, Copy)]
pub struct CommandFmt ;

pub type CommandFmtSpec = Named < Mapped < Bind < U8, spec_fn (u8) -> CommandBodyFmt >, BiMap < CommandForward, CommandReverse >> > ;

impl CommandFmt {
    # [doc = "specification constructor for `command`."] pub open spec fn spec_inner() -> CommandFmtSpec {
        Named ("command",
        Mapped {
            inner: Bind (U8,
            | opcode: u8 | CommandBodyFmt::spec (opcode)),
            mapper: BiMap (CommandForward,
            CommandReverse),
        }
        )
    }
}


# [doc = "named format combinator for `command_body`."]
# [derive (Clone, Copy)]
pub struct CommandBodyFmt {
    opcode: u8,
}
impl CommandBodyFmt {
    # [verifier::type_invariant] spec fn wf (& self) -> bool {
        true
    }
    pub closed spec fn opcode_spec (& self) -> u8 {
        self.opcode.deep_view()
    }
    pub closed spec fn spec (opcode: u8) -> Self {
        CommandBodyFmt {
            opcode
        }
    }
}

pub type CommandBodyFmtSpec = Named < Mapped < Sum < Sum < Sum < Sum < Empty, Varied < u8 > >, Sum < Literal1Fmt, Sum < Literal2Fmt, Literal4Fmt > > >, Sum < Sum < Literal8Fmt, Sum < Copy11Fmt, Copy12Fmt > >, Sum < Copy14Fmt, Sum < Copy18Fmt, Copy21Fmt > > > >, Sum < Sum < Sum < Copy22Fmt, Sum < Copy24Fmt, Copy28Fmt > >, Sum < Copy41Fmt, Sum < Copy42Fmt, Copy44Fmt > > >, Sum < Sum < Copy48Fmt, Sum < Copy81Fmt, Copy82Fmt > >, Sum < Copy84Fmt, Sum < Copy88Fmt, Void > > > > >, BiMap < CommandBodyForward, CommandBodyReverse >> > ;

impl CommandBodyFmt {
    # [doc = "specification constructor for `command_body`."] pub open spec fn spec_inner (opcode: u8) -> CommandBodyFmtSpec {
        Named ("command_body",
        Mapped {
            inner: match opcode {
                0 => L (L (L (L (Empty)))),
                1 ..= 64 => L (L (L (R (Varied (opcode))))),
                65 => L (L (R (L (Literal1Fmt)))),
                66 => L (L (R (R (L (Literal2Fmt))))),
                67 => L (L (R (R (R (Literal4Fmt))))),
                68 => L (R (L (L (Literal8Fmt)))),
                69 => L (R (L (R (L (Copy11Fmt))))),
                70 => L (R (L (R (R (Copy12Fmt))))),
                71 => L (R (R (L (Copy14Fmt)))),
                72 => L (R (R (R (L (Copy18Fmt))))),
                73 => L (R (R (R (R (Copy21Fmt))))),
                74 => R (L (L (L (Copy22Fmt)))),
                75 => R (L (L (R (L (Copy24Fmt))))),
                76 => R (L (L (R (R (Copy28Fmt))))),
                77 => R (L (R (L (Copy41Fmt)))),
                78 => R (L (R (R (L (Copy42Fmt))))),
                79 => R (L (R (R (R (Copy44Fmt))))),
                80 => R (R (L (L (Copy48Fmt)))),
                81 => R (R (L (R (L (Copy81Fmt))))),
                82 => R (R (L (R (R (Copy82Fmt))))),
                83 => R (R (R (L (Copy84Fmt)))),
                84 => R (R (R (R (L (Copy88Fmt))))),
                _ => R (R (R (R (R (Void ("reserved librsync delta opcode")))))),
            }
           ,
            mapper: BiMap (CommandBodyForward,
            CommandBodyReverse),
        }
        )
    }
}

// ============================================================
// Derived Parser, Serializer, Length, and Consistency Specifications
// ============================================================
mod derived_specs {
    use super::*;

    impl SpecParser for DeltaMagicFmt {
        type PVal = DeltaMagicSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for DeltaMagicFmt {
        type Val = DeltaMagicSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for DeltaMagicFmt {
        type SValue = DeltaMagicSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for DeltaMagicFmt {
        type SVal = DeltaMagicSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for DeltaMagicFmt {
        type T = DeltaMagicSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Literal1Fmt {
        type PVal = Literal1Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Literal1Fmt {
        type Val = Literal1Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Literal1Fmt {
        type SValue = Literal1Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Literal1Fmt {
        type SVal = Literal1Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Literal1Fmt {
        type T = Literal1Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Literal2Fmt {
        type PVal = Literal2Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Literal2Fmt {
        type Val = Literal2Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Literal2Fmt {
        type SValue = Literal2Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Literal2Fmt {
        type SVal = Literal2Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Literal2Fmt {
        type T = Literal2Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Literal4Fmt {
        type PVal = Literal4Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Literal4Fmt {
        type Val = Literal4Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Literal4Fmt {
        type SValue = Literal4Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Literal4Fmt {
        type SVal = Literal4Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Literal4Fmt {
        type T = Literal4Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Literal8Fmt {
        type PVal = Literal8Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Literal8Fmt {
        type Val = Literal8Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Literal8Fmt {
        type SValue = Literal8Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Literal8Fmt {
        type SVal = Literal8Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Literal8Fmt {
        type T = Literal8Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy11Fmt {
        type PVal = Copy11Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy11Fmt {
        type Val = Copy11Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy11Fmt {
        type SValue = Copy11Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy11Fmt {
        type SVal = Copy11Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy11Fmt {
        type T = Copy11Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy12Fmt {
        type PVal = Copy12Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy12Fmt {
        type Val = Copy12Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy12Fmt {
        type SValue = Copy12Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy12Fmt {
        type SVal = Copy12Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy12Fmt {
        type T = Copy12Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy14Fmt {
        type PVal = Copy14Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy14Fmt {
        type Val = Copy14Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy14Fmt {
        type SValue = Copy14Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy14Fmt {
        type SVal = Copy14Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy14Fmt {
        type T = Copy14Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy18Fmt {
        type PVal = Copy18Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy18Fmt {
        type Val = Copy18Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy18Fmt {
        type SValue = Copy18Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy18Fmt {
        type SVal = Copy18Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy18Fmt {
        type T = Copy18Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy21Fmt {
        type PVal = Copy21Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy21Fmt {
        type Val = Copy21Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy21Fmt {
        type SValue = Copy21Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy21Fmt {
        type SVal = Copy21Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy21Fmt {
        type T = Copy21Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy22Fmt {
        type PVal = Copy22Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy22Fmt {
        type Val = Copy22Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy22Fmt {
        type SValue = Copy22Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy22Fmt {
        type SVal = Copy22Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy22Fmt {
        type T = Copy22Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy24Fmt {
        type PVal = Copy24Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy24Fmt {
        type Val = Copy24Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy24Fmt {
        type SValue = Copy24Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy24Fmt {
        type SVal = Copy24Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy24Fmt {
        type T = Copy24Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy28Fmt {
        type PVal = Copy28Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy28Fmt {
        type Val = Copy28Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy28Fmt {
        type SValue = Copy28Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy28Fmt {
        type SVal = Copy28Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy28Fmt {
        type T = Copy28Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy41Fmt {
        type PVal = Copy41Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy41Fmt {
        type Val = Copy41Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy41Fmt {
        type SValue = Copy41Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy41Fmt {
        type SVal = Copy41Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy41Fmt {
        type T = Copy41Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy42Fmt {
        type PVal = Copy42Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy42Fmt {
        type Val = Copy42Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy42Fmt {
        type SValue = Copy42Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy42Fmt {
        type SVal = Copy42Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy42Fmt {
        type T = Copy42Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy44Fmt {
        type PVal = Copy44Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy44Fmt {
        type Val = Copy44Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy44Fmt {
        type SValue = Copy44Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy44Fmt {
        type SVal = Copy44Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy44Fmt {
        type T = Copy44Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy48Fmt {
        type PVal = Copy48Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy48Fmt {
        type Val = Copy48Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy48Fmt {
        type SValue = Copy48Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy48Fmt {
        type SVal = Copy48Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy48Fmt {
        type T = Copy48Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy81Fmt {
        type PVal = Copy81Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy81Fmt {
        type Val = Copy81Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy81Fmt {
        type SValue = Copy81Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy81Fmt {
        type SVal = Copy81Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy81Fmt {
        type T = Copy81Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy82Fmt {
        type PVal = Copy82Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy82Fmt {
        type Val = Copy82Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy82Fmt {
        type SValue = Copy82Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy82Fmt {
        type SVal = Copy82Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy82Fmt {
        type T = Copy82Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy84Fmt {
        type PVal = Copy84Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy84Fmt {
        type Val = Copy84Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy84Fmt {
        type SValue = Copy84Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy84Fmt {
        type SVal = Copy84Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy84Fmt {
        type T = Copy84Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for Copy88Fmt {
        type PVal = Copy88Spec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for Copy88Fmt {
        type Val = Copy88Spec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for Copy88Fmt {
        type SValue = Copy88Spec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for Copy88Fmt {
        type SVal = Copy88Spec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for Copy88Fmt {
        type T = Copy88Spec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for CommandFmt {
        type PVal = CommandSpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner().spec_parse (ibuf)
        }
    }
    impl Consistency for CommandFmt {
        type Val = CommandSpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner().consistent (v)
        }
    }
    impl SpecSerializerDps for CommandFmt {
        type SValue = CommandSpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner().spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for CommandFmt {
        type SVal = CommandSpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner().spec_serialize (v)
        }
    }
    impl SpecByteLen for CommandFmt {
        type T = CommandSpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner().byte_len (v)
        }
    }

    impl SpecParser for CommandBodyFmt {
        type PVal = CommandBodySpec ;
        # [verifier::opaque] open spec fn spec_parse (& self,
        ibuf: Seq < u8 >) -> Option < (int,
        Self::PVal) > {
            Self::spec_inner (self.opcode_spec()).spec_parse (ibuf)
        }
    }
    impl Consistency for CommandBodyFmt {
        type Val = CommandBodySpec ;
        open spec fn consistent (& self,
        v: Self::Val) -> bool {
            Self::spec_inner (self.opcode_spec()).consistent (v)
        }
    }
    impl SpecSerializerDps for CommandBodyFmt {
        type SValue = CommandBodySpec ;
        # [verifier::opaque] open spec fn spec_serialize_dps (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) -> Seq < u8 > {
            Self::spec_inner (self.opcode_spec()).spec_serialize_dps (v,
            obuf)
        }
    }
    impl SpecSerializer for CommandBodyFmt {
        type SVal = CommandBodySpec ;
        # [verifier::opaque] open spec fn spec_serialize (& self,
        v: Self::SVal) -> Seq < u8 > {
            Self::spec_inner (self.opcode_spec()).spec_serialize (v)
        }
    }
    impl SpecByteLen for CommandBodyFmt {
        type T = CommandBodySpec ;
        # [verifier::opaque] open spec fn byte_len (& self,
        v: Self::T) -> nat {
            Self::spec_inner (self.opcode_spec()).byte_len (v)
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
        Literal1Spec::lemma_from_into,
        Literal1Spec::lemma_into_from,
        Literal2Spec::lemma_from_into,
        Literal2Spec::lemma_into_from,
        Literal4Spec::lemma_from_into,
        Literal4Spec::lemma_into_from,
        Literal8Spec::lemma_from_into,
        Literal8Spec::lemma_into_from,
        Copy11Spec::lemma_from_into,
        Copy11Spec::lemma_into_from,
        Copy12Spec::lemma_from_into,
        Copy12Spec::lemma_into_from,
        Copy14Spec::lemma_from_into,
        Copy14Spec::lemma_into_from,
        Copy18Spec::lemma_from_into,
        Copy18Spec::lemma_into_from,
        Copy21Spec::lemma_from_into,
        Copy21Spec::lemma_into_from,
        Copy22Spec::lemma_from_into,
        Copy22Spec::lemma_into_from,
        Copy24Spec::lemma_from_into,
        Copy24Spec::lemma_into_from,
        Copy28Spec::lemma_from_into,
        Copy28Spec::lemma_into_from,
        Copy41Spec::lemma_from_into,
        Copy41Spec::lemma_into_from,
        Copy42Spec::lemma_from_into,
        Copy42Spec::lemma_into_from,
        Copy44Spec::lemma_from_into,
        Copy44Spec::lemma_into_from,
        Copy48Spec::lemma_from_into,
        Copy48Spec::lemma_into_from,
        Copy81Spec::lemma_from_into,
        Copy81Spec::lemma_into_from,
        Copy82Spec::lemma_from_into,
        Copy82Spec::lemma_into_from,
        Copy84Spec::lemma_from_into,
        Copy84Spec::lemma_into_from,
        Copy88Spec::lemma_from_into,
        Copy88Spec::lemma_into_from,
        CommandSpec::lemma_from_into,
        CommandSpec::lemma_into_from,
        CommandBodySpec::lemma_from_into,
        CommandBodySpec::lemma_into_from,
    };

    impl SafeParser for DeltaMagicFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for DeltaMagicFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for DeltaMagicFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            reveal(< DeltaMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            reveal(< DeltaMagicFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for DeltaMagicFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DeltaMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for DeltaMagicFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< DeltaMagicFmt as SpecSerializer>::spec_serialize) ;
            reveal(< DeltaMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for DeltaMagicFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            reveal(< DeltaMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DeltaMagicFmt as Consistency>::consistent) ;
            reveal(< DeltaMagicFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for DeltaMagicFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for DeltaMagicFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< DeltaMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DeltaMagicFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for DeltaMagicFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< DeltaMagicFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< DeltaMagicFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Literal1Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Literal1Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Literal1Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal1Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal1Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal1Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal1Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal1Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal1Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Literal1Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal1Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Literal1Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Literal1Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Literal1Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Literal1Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal1Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal1Fmt as Consistency>::consistent) ;
            reveal(< Literal1Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Literal1Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Literal1Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Literal1Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal1Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal1Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Literal1Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Literal1Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal1Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Literal1Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Literal1Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal1Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Literal2Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Literal2Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Literal2Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal2Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal2Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal2Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal2Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal2Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal2Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Literal2Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal2Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Literal2Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Literal2Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Literal2Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Literal2Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal2Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal2Fmt as Consistency>::consistent) ;
            reveal(< Literal2Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Literal2Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Literal2Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Literal2Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal2Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal2Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Literal2Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Literal2Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal2Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Literal2Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Literal2Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal2Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Literal4Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Literal4Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Literal4Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal4Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal4Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal4Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal4Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal4Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal4Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Literal4Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal4Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Literal4Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Literal4Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Literal4Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Literal4Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal4Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal4Fmt as Consistency>::consistent) ;
            reveal(< Literal4Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Literal4Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Literal4Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Literal4Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal4Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal4Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Literal4Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Literal4Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal4Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Literal4Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Literal4Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal4Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Literal8Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Literal8Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Literal8Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal8Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal8Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal8Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal8Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal8Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal8Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Literal8Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal8Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Literal8Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Literal8Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Literal8Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Literal8Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            reveal(< Literal8Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal8Fmt as Consistency>::consistent) ;
            reveal(< Literal8Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Literal8Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Literal8Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Literal8Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Literal8Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Literal8Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Literal8Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Literal8Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal8Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Literal8Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Literal8Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Literal8Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy11Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy11Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy11Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy11Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy11Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy11Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy11Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy11Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy11Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy11Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy11Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy11Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy11Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy11Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy11Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy11Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy11Fmt as Consistency>::consistent) ;
            reveal(< Copy11Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy11Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy11Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy11Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy11Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy11Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy11Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy11Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy11Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy11Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy11Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy11Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy12Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy12Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy12Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy12Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy12Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy12Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy12Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy12Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy12Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy12Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy12Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy12Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy12Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy12Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy12Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy12Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy12Fmt as Consistency>::consistent) ;
            reveal(< Copy12Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy12Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy12Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy12Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy12Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy12Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy12Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy12Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy12Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy12Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy12Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy12Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy14Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy14Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy14Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy14Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy14Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy14Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy14Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy14Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy14Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy14Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy14Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy14Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy14Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy14Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy14Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy14Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy14Fmt as Consistency>::consistent) ;
            reveal(< Copy14Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy14Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy14Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy14Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy14Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy14Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy14Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy14Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy14Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy14Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy14Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy14Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy18Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy18Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy18Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy18Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy18Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy18Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy18Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy18Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy18Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy18Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy18Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy18Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy18Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy18Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy18Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy18Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy18Fmt as Consistency>::consistent) ;
            reveal(< Copy18Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy18Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy18Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy18Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy18Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy18Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy18Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy18Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy18Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy18Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy18Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy18Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy21Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy21Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy21Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy21Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy21Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy21Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy21Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy21Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy21Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy21Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy21Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy21Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy21Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy21Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy21Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy21Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy21Fmt as Consistency>::consistent) ;
            reveal(< Copy21Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy21Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy21Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy21Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy21Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy21Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy21Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy21Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy21Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy21Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy21Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy21Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy22Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy22Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy22Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy22Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy22Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy22Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy22Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy22Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy22Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy22Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy22Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy22Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy22Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy22Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy22Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy22Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy22Fmt as Consistency>::consistent) ;
            reveal(< Copy22Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy22Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy22Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy22Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy22Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy22Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy22Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy22Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy22Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy22Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy22Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy22Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy24Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy24Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy24Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy24Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy24Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy24Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy24Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy24Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy24Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy24Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy24Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy24Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy24Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy24Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy24Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy24Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy24Fmt as Consistency>::consistent) ;
            reveal(< Copy24Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy24Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy24Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy24Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy24Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy24Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy24Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy24Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy24Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy24Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy24Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy24Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy28Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy28Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy28Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy28Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy28Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy28Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy28Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy28Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy28Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy28Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy28Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy28Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy28Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy28Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy28Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy28Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy28Fmt as Consistency>::consistent) ;
            reveal(< Copy28Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy28Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy28Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy28Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy28Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy28Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy28Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy28Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy28Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy28Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy28Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy28Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy41Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy41Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy41Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy41Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy41Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy41Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy41Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy41Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy41Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy41Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy41Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy41Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy41Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy41Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy41Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy41Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy41Fmt as Consistency>::consistent) ;
            reveal(< Copy41Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy41Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy41Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy41Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy41Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy41Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy41Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy41Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy41Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy41Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy41Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy41Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy42Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy42Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy42Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy42Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy42Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy42Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy42Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy42Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy42Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy42Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy42Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy42Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy42Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy42Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy42Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy42Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy42Fmt as Consistency>::consistent) ;
            reveal(< Copy42Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy42Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy42Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy42Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy42Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy42Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy42Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy42Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy42Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy42Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy42Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy42Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy44Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy44Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy44Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy44Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy44Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy44Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy44Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy44Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy44Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy44Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy44Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy44Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy44Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy44Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy44Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy44Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy44Fmt as Consistency>::consistent) ;
            reveal(< Copy44Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy44Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy44Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy44Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy44Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy44Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy44Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy44Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy44Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy44Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy44Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy44Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy48Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy48Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy48Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy48Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy48Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy48Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy48Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy48Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy48Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy48Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy48Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy48Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy48Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy48Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy48Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy48Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy48Fmt as Consistency>::consistent) ;
            reveal(< Copy48Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy48Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy48Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy48Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy48Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy48Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy48Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy48Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy48Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy48Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy48Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy48Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy81Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy81Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy81Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy81Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy81Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy81Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy81Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy81Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy81Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy81Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy81Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy81Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy81Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy81Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy81Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy81Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy81Fmt as Consistency>::consistent) ;
            reveal(< Copy81Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy81Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy81Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy81Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy81Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy81Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy81Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy81Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy81Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy81Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy81Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy81Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy82Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy82Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy82Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy82Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy82Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy82Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy82Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy82Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy82Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy82Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy82Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy82Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy82Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy82Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy82Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy82Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy82Fmt as Consistency>::consistent) ;
            reveal(< Copy82Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy82Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy82Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy82Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy82Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy82Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy82Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy82Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy82Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy82Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy82Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy82Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy84Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy84Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy84Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy84Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy84Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy84Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy84Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy84Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy84Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy84Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy84Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy84Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy84Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy84Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy84Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy84Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy84Fmt as Consistency>::consistent) ;
            reveal(< Copy84Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy84Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy84Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy84Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy84Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy84Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy84Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy84Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy84Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy84Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy84Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy84Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for Copy88Fmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for Copy88Fmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for Copy88Fmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy88Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy88Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy88Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy88Fmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy88Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy88Spec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for Copy88Fmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy88Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for Copy88Fmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< Copy88Fmt as SpecSerializer>::spec_serialize) ;
            reveal(< Copy88Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for Copy88Fmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            reveal(< Copy88Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy88Fmt as Consistency>::consistent) ;
            reveal(< Copy88Fmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: Copy88Spec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                Copy88Spec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for Copy88Fmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: Copy88Inner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                Copy88Spec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for Copy88Fmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< Copy88Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy88Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for Copy88Fmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< Copy88Fmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< Copy88Fmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for CommandFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            Self::spec_inner().lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for CommandFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner().productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for CommandFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            reveal(< CommandFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CommandInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            reveal(< CommandFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CommandInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandSpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for CommandFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for CommandFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< CommandFmt as SpecSerializer>::spec_serialize) ;
            reveal(< CommandFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for CommandFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            reveal(< CommandFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandFmt as Consistency>::consistent) ;
            reveal(< CommandFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner() ;
            assert forall | output: CommandSpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                CommandSpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for CommandFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< CommandFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner() ;
            assert forall | input: CommandInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandSpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for CommandFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< CommandFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for CommandFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< CommandFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner() ;
            assert (fmt.equiv_inv()) ;
            fmt.lemma_serialize_equiv_on_empty (v) ;
        }
    }

    impl SafeParser for CommandBodyFmt {
        proof fn lemma_parse_safe (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            Self::spec_inner (self.opcode_spec()).lemma_parse_safe (ibuf) ;
        }
    }
    impl Productive for CommandBodyFmt {
        open spec fn productive_inv (& self) -> bool {
            Self::spec_inner (self.opcode_spec()).productive_inv()
        }
        proof fn lemma_productive (& self,
        s: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert (fmt.productive_inv()) ;
            fmt.lemma_productive (s) ;
        }
    }
    impl SoundParser for CommandBodyFmt {
        proof fn lemma_parse_sound_consumption (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            reveal(< CommandBodyFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert forall | input: CommandBodyInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandBodySpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_consumption (ibuf) ;
        }
        proof fn lemma_parse_sound_value (& self,
        ibuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            reveal(< CommandBodyFmt as Consistency>::consistent) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert forall | input: CommandBodyInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandBodySpec::lemma_into_from (input) ;
            }
            assert (fmt.sound_inv()) ;
            fmt.lemma_parse_sound_value (ibuf) ;
        }
    }
    impl NonTailFmt for CommandBodyFmt {
        proof fn lemma_serialize_dps_prepend (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecSerializerDps>::spec_serialize_dps) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_prepend (v,
            obuf) ;
        }
        proof fn lemma_serialize_dps_len (& self,
        v: Self::SValue,
        obuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandBodyFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert (fmt.serialize_dps_inv()) ;
            fmt.lemma_serialize_dps_len (v,
            obuf) ;
        }
    }
    impl GoodSerializer for CommandBodyFmt {
        proof fn lemma_serialize_len (& self,
        v: Self::SVal) {
            reveal(< CommandBodyFmt as SpecSerializer>::spec_serialize) ;
            reveal(< CommandBodyFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert (fmt.serialize_inv()) ;
            fmt.lemma_serialize_len (v) ;
        }
    }
    impl SPRoundTripDps for CommandBodyFmt {
        proof fn theorem_serialize_dps_parse_roundtrip (& self,
        v: Self::T,
        obuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            reveal(< CommandBodyFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandBodyFmt as Consistency>::consistent) ;
            reveal(< CommandBodyFmt as SpecByteLen>::byte_len) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert forall | output: CommandBodySpec | # [trigger] fmt.1.consistent (output) implies fmt.1.mapper.sound (output) by {
                CommandBodySpec::lemma_from_into (output) ;
            }
            assert (fmt.unambiguous()) ;
            fmt.theorem_serialize_dps_parse_roundtrip (v,
            obuf) ;
        }
    }
    impl NonMalleable for CommandBodyFmt {
        proof fn lemma_parse_non_malleable (& self,
        buf1: Seq < u8 >,
        buf2: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecParser>::spec_parse) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert forall | input: CommandBodyInner | # [trigger] fmt.1.inner.consistent (input) implies fmt.1.mapper.lossless (input) by {
                CommandBodySpec::lemma_into_from (input) ;
            }
            assert (fmt.nonmal_inv()) ;
            fmt.lemma_parse_non_malleable (buf1,
            buf2) ;
        }
    }
    impl EquivSerializersGeneral for CommandBodyFmt {
        proof fn lemma_serialize_equiv (& self,
        v: Self::SVal,
        obuf: Seq < u8 >) {
            reveal(< CommandBodyFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandBodyFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
            assert (fmt.equiv_general_inv()) ;
            fmt.lemma_serialize_equiv (v,
            obuf) ;
        }
    }
    impl EquivSerializers for CommandBodyFmt {
        proof fn lemma_serialize_equiv_on_empty (& self,
        v: Self::SVal) {
            reveal(< CommandBodyFmt as SpecSerializerDps>::spec_serialize_dps) ;
            reveal(< CommandBodyFmt as SpecSerializer>::spec_serialize) ;
            let fmt = Self::spec_inner (self.opcode_spec()) ;
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

    impl<'i> Parser<&'i [u8]> for DeltaMagicFmt {
        type PT = DeltaMagic;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<DeltaMagicFmt as SpecParser>::spec_parse);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n, v) = U32Be.parse(ibuf)?;
            if !(v == 1920139830) {
                return Err(ParseError::predicate_failed());
            }
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, DeltaMagic> for DeltaMagicFmt {
        fn serialize_into(&self, v: &DeltaMagic, obuf: &mut Output) {
            reveal(<DeltaMagicFmt as SpecSerializer>::spec_serialize);
            reveal(<DeltaMagicFmt as SpecByteLen>::byte_len);
            let ghost old_obuf = obuf@;

            U32Be.serialize_into(v, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<DeltaMagic> for DeltaMagicFmt {
        fn prepare(&self, v: &DeltaMagic) -> Result<usize, PreSerializeError> {
            reveal(<DeltaMagicFmt as SpecByteLen>::byte_len);
            {
        if ! (*v == 1920139830) {
            Err (PreSerializeError::not_compliant (ComplianceErrorKind::PredicateFailed))
        }
        else {
            (U32Be).prepare (v)
        }
    }
        }
    }



    impl<'i> Parser<&'i [u8]> for Literal1Fmt {
        type PT = Literal1<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Literal1Fmt as SpecParser>::spec_parse);
            reveal(<Literal1 as DeepView>::deep_view);
            reveal(Literal1Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, length) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, data) = (Varied (length)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Literal1 {
                length,
                data,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Literal1<'i>> for Literal1Fmt {
        fn serialize_into(&self, v: &Literal1<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Literal1Fmt as SpecSerializer>::spec_serialize);
            reveal(<Literal1Fmt as SpecByteLen>::byte_len);
            reveal(<Literal1 as DeepView>::deep_view);
            reveal(Literal1Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Literal1 {
                length,
                data,
            } = v;
            U8.serialize_into(length, obuf);
            Varied (*length).serialize_into(* data, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Literal1<'i>> for Literal1Fmt {
        fn prepare(&self, v: &Literal1<'i>) -> Result<usize, PreSerializeError> {
            reveal(<Literal1Fmt as SpecByteLen>::byte_len);
            reveal(<Literal1 as DeepView>::deep_view);
            reveal(Literal1Spec::into_structural);
            let Literal1 {
                length,
                data,
            } = v;
            let l1 = (U8).prepare (length) ?;
            let l2 = (Varied (*length)).prepare (data) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Literal2Fmt {
        type PT = Literal2<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Literal2Fmt as SpecParser>::spec_parse);
            reveal(<Literal2 as DeepView>::deep_view);
            reveal(Literal2Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, length) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, data) = (Varied (length)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Literal2 {
                length,
                data,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Literal2<'i>> for Literal2Fmt {
        fn serialize_into(&self, v: &Literal2<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Literal2Fmt as SpecSerializer>::spec_serialize);
            reveal(<Literal2Fmt as SpecByteLen>::byte_len);
            reveal(<Literal2 as DeepView>::deep_view);
            reveal(Literal2Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Literal2 {
                length,
                data,
            } = v;
            U16Be.serialize_into(length, obuf);
            Varied (*length).serialize_into(* data, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Literal2<'i>> for Literal2Fmt {
        fn prepare(&self, v: &Literal2<'i>) -> Result<usize, PreSerializeError> {
            reveal(<Literal2Fmt as SpecByteLen>::byte_len);
            reveal(<Literal2 as DeepView>::deep_view);
            reveal(Literal2Spec::into_structural);
            let Literal2 {
                length,
                data,
            } = v;
            let l1 = (U16Be).prepare (length) ?;
            let l2 = (Varied (*length)).prepare (data) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Literal4Fmt {
        type PT = Literal4<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Literal4Fmt as SpecParser>::spec_parse);
            reveal(<Literal4 as DeepView>::deep_view);
            reveal(Literal4Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, data) = (Varied (length)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Literal4 {
                length,
                data,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Literal4<'i>> for Literal4Fmt {
        fn serialize_into(&self, v: &Literal4<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Literal4Fmt as SpecSerializer>::spec_serialize);
            reveal(<Literal4Fmt as SpecByteLen>::byte_len);
            reveal(<Literal4 as DeepView>::deep_view);
            reveal(Literal4Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Literal4 {
                length,
                data,
            } = v;
            U32Be.serialize_into(length, obuf);
            Varied (*length).serialize_into(* data, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Literal4<'i>> for Literal4Fmt {
        fn prepare(&self, v: &Literal4<'i>) -> Result<usize, PreSerializeError> {
            reveal(<Literal4Fmt as SpecByteLen>::byte_len);
            reveal(<Literal4 as DeepView>::deep_view);
            reveal(Literal4Spec::into_structural);
            let Literal4 {
                length,
                data,
            } = v;
            let l1 = (U32Be).prepare (length) ?;
            let l2 = (Varied (*length)).prepare (data) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Literal8Fmt {
        type PT = Literal8<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Literal8Fmt as SpecParser>::spec_parse);
            reveal(<Literal8 as DeepView>::deep_view);
            reveal(Literal8Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, length) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, data) = (Varied (length)).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Literal8 {
                length,
                data,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Literal8<'i>> for Literal8Fmt {
        fn serialize_into(&self, v: &Literal8<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Literal8Fmt as SpecSerializer>::spec_serialize);
            reveal(<Literal8Fmt as SpecByteLen>::byte_len);
            reveal(<Literal8 as DeepView>::deep_view);
            reveal(Literal8Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Literal8 {
                length,
                data,
            } = v;
            U64Be.serialize_into(length, obuf);
            Varied (*length).serialize_into(* data, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Literal8<'i>> for Literal8Fmt {
        fn prepare(&self, v: &Literal8<'i>) -> Result<usize, PreSerializeError> {
            reveal(<Literal8Fmt as SpecByteLen>::byte_len);
            reveal(<Literal8 as DeepView>::deep_view);
            reveal(Literal8Spec::into_structural);
            let Literal8 {
                length,
                data,
            } = v;
            let l1 = (U64Be).prepare (length) ?;
            let l2 = (Varied (*length)).prepare (data) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy11Fmt {
        type PT = Copy11;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy11Fmt as SpecParser>::spec_parse);
            reveal(<Copy11 as DeepView>::deep_view);
            reveal(Copy11Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U8).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy11 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy11> for Copy11Fmt {
        fn serialize_into(&self, v: &Copy11, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy11Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy11Fmt as SpecByteLen>::byte_len);
            reveal(<Copy11 as DeepView>::deep_view);
            reveal(Copy11Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy11 {
                offset,
                length,
            } = v;
            U8.serialize_into(offset, obuf);
            U8.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy11> for Copy11Fmt {
        fn prepare(&self, v: &Copy11) -> Result<usize, PreSerializeError> {
            reveal(<Copy11Fmt as SpecByteLen>::byte_len);
            reveal(<Copy11 as DeepView>::deep_view);
            reveal(Copy11Spec::into_structural);
            let Copy11 {
                offset,
                length,
            } = v;
            let l1 = (U8).prepare (offset) ?;
            let l2 = (U8).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy12Fmt {
        type PT = Copy12;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy12Fmt as SpecParser>::spec_parse);
            reveal(<Copy12 as DeepView>::deep_view);
            reveal(Copy12Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy12 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy12> for Copy12Fmt {
        fn serialize_into(&self, v: &Copy12, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy12Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy12Fmt as SpecByteLen>::byte_len);
            reveal(<Copy12 as DeepView>::deep_view);
            reveal(Copy12Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy12 {
                offset,
                length,
            } = v;
            U8.serialize_into(offset, obuf);
            U16Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy12> for Copy12Fmt {
        fn prepare(&self, v: &Copy12) -> Result<usize, PreSerializeError> {
            reveal(<Copy12Fmt as SpecByteLen>::byte_len);
            reveal(<Copy12 as DeepView>::deep_view);
            reveal(Copy12Spec::into_structural);
            let Copy12 {
                offset,
                length,
            } = v;
            let l1 = (U8).prepare (offset) ?;
            let l2 = (U16Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy14Fmt {
        type PT = Copy14;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy14Fmt as SpecParser>::spec_parse);
            reveal(<Copy14 as DeepView>::deep_view);
            reveal(Copy14Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy14 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy14> for Copy14Fmt {
        fn serialize_into(&self, v: &Copy14, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy14Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy14Fmt as SpecByteLen>::byte_len);
            reveal(<Copy14 as DeepView>::deep_view);
            reveal(Copy14Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy14 {
                offset,
                length,
            } = v;
            U8.serialize_into(offset, obuf);
            U32Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy14> for Copy14Fmt {
        fn prepare(&self, v: &Copy14) -> Result<usize, PreSerializeError> {
            reveal(<Copy14Fmt as SpecByteLen>::byte_len);
            reveal(<Copy14 as DeepView>::deep_view);
            reveal(Copy14Spec::into_structural);
            let Copy14 {
                offset,
                length,
            } = v;
            let l1 = (U8).prepare (offset) ?;
            let l2 = (U32Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy18Fmt {
        type PT = Copy18;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy18Fmt as SpecParser>::spec_parse);
            reveal(<Copy18 as DeepView>::deep_view);
            reveal(Copy18Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy18 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy18> for Copy18Fmt {
        fn serialize_into(&self, v: &Copy18, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy18Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy18Fmt as SpecByteLen>::byte_len);
            reveal(<Copy18 as DeepView>::deep_view);
            reveal(Copy18Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy18 {
                offset,
                length,
            } = v;
            U8.serialize_into(offset, obuf);
            U64Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy18> for Copy18Fmt {
        fn prepare(&self, v: &Copy18) -> Result<usize, PreSerializeError> {
            reveal(<Copy18Fmt as SpecByteLen>::byte_len);
            reveal(<Copy18 as DeepView>::deep_view);
            reveal(Copy18Spec::into_structural);
            let Copy18 {
                offset,
                length,
            } = v;
            let l1 = (U8).prepare (offset) ?;
            let l2 = (U64Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy21Fmt {
        type PT = Copy21;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy21Fmt as SpecParser>::spec_parse);
            reveal(<Copy21 as DeepView>::deep_view);
            reveal(Copy21Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U8).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy21 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy21> for Copy21Fmt {
        fn serialize_into(&self, v: &Copy21, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy21Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy21Fmt as SpecByteLen>::byte_len);
            reveal(<Copy21 as DeepView>::deep_view);
            reveal(Copy21Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy21 {
                offset,
                length,
            } = v;
            U16Be.serialize_into(offset, obuf);
            U8.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy21> for Copy21Fmt {
        fn prepare(&self, v: &Copy21) -> Result<usize, PreSerializeError> {
            reveal(<Copy21Fmt as SpecByteLen>::byte_len);
            reveal(<Copy21 as DeepView>::deep_view);
            reveal(Copy21Spec::into_structural);
            let Copy21 {
                offset,
                length,
            } = v;
            let l1 = (U16Be).prepare (offset) ?;
            let l2 = (U8).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy22Fmt {
        type PT = Copy22;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy22Fmt as SpecParser>::spec_parse);
            reveal(<Copy22 as DeepView>::deep_view);
            reveal(Copy22Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy22 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy22> for Copy22Fmt {
        fn serialize_into(&self, v: &Copy22, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy22Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy22Fmt as SpecByteLen>::byte_len);
            reveal(<Copy22 as DeepView>::deep_view);
            reveal(Copy22Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy22 {
                offset,
                length,
            } = v;
            U16Be.serialize_into(offset, obuf);
            U16Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy22> for Copy22Fmt {
        fn prepare(&self, v: &Copy22) -> Result<usize, PreSerializeError> {
            reveal(<Copy22Fmt as SpecByteLen>::byte_len);
            reveal(<Copy22 as DeepView>::deep_view);
            reveal(Copy22Spec::into_structural);
            let Copy22 {
                offset,
                length,
            } = v;
            let l1 = (U16Be).prepare (offset) ?;
            let l2 = (U16Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy24Fmt {
        type PT = Copy24;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy24Fmt as SpecParser>::spec_parse);
            reveal(<Copy24 as DeepView>::deep_view);
            reveal(Copy24Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy24 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy24> for Copy24Fmt {
        fn serialize_into(&self, v: &Copy24, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy24Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy24Fmt as SpecByteLen>::byte_len);
            reveal(<Copy24 as DeepView>::deep_view);
            reveal(Copy24Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy24 {
                offset,
                length,
            } = v;
            U16Be.serialize_into(offset, obuf);
            U32Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy24> for Copy24Fmt {
        fn prepare(&self, v: &Copy24) -> Result<usize, PreSerializeError> {
            reveal(<Copy24Fmt as SpecByteLen>::byte_len);
            reveal(<Copy24 as DeepView>::deep_view);
            reveal(Copy24Spec::into_structural);
            let Copy24 {
                offset,
                length,
            } = v;
            let l1 = (U16Be).prepare (offset) ?;
            let l2 = (U32Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy28Fmt {
        type PT = Copy28;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy28Fmt as SpecParser>::spec_parse);
            reveal(<Copy28 as DeepView>::deep_view);
            reveal(Copy28Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy28 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy28> for Copy28Fmt {
        fn serialize_into(&self, v: &Copy28, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy28Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy28Fmt as SpecByteLen>::byte_len);
            reveal(<Copy28 as DeepView>::deep_view);
            reveal(Copy28Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy28 {
                offset,
                length,
            } = v;
            U16Be.serialize_into(offset, obuf);
            U64Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy28> for Copy28Fmt {
        fn prepare(&self, v: &Copy28) -> Result<usize, PreSerializeError> {
            reveal(<Copy28Fmt as SpecByteLen>::byte_len);
            reveal(<Copy28 as DeepView>::deep_view);
            reveal(Copy28Spec::into_structural);
            let Copy28 {
                offset,
                length,
            } = v;
            let l1 = (U16Be).prepare (offset) ?;
            let l2 = (U64Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy41Fmt {
        type PT = Copy41;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy41Fmt as SpecParser>::spec_parse);
            reveal(<Copy41 as DeepView>::deep_view);
            reveal(Copy41Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U8).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy41 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy41> for Copy41Fmt {
        fn serialize_into(&self, v: &Copy41, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy41Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy41Fmt as SpecByteLen>::byte_len);
            reveal(<Copy41 as DeepView>::deep_view);
            reveal(Copy41Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy41 {
                offset,
                length,
            } = v;
            U32Be.serialize_into(offset, obuf);
            U8.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy41> for Copy41Fmt {
        fn prepare(&self, v: &Copy41) -> Result<usize, PreSerializeError> {
            reveal(<Copy41Fmt as SpecByteLen>::byte_len);
            reveal(<Copy41 as DeepView>::deep_view);
            reveal(Copy41Spec::into_structural);
            let Copy41 {
                offset,
                length,
            } = v;
            let l1 = (U32Be).prepare (offset) ?;
            let l2 = (U8).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy42Fmt {
        type PT = Copy42;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy42Fmt as SpecParser>::spec_parse);
            reveal(<Copy42 as DeepView>::deep_view);
            reveal(Copy42Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy42 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy42> for Copy42Fmt {
        fn serialize_into(&self, v: &Copy42, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy42Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy42Fmt as SpecByteLen>::byte_len);
            reveal(<Copy42 as DeepView>::deep_view);
            reveal(Copy42Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy42 {
                offset,
                length,
            } = v;
            U32Be.serialize_into(offset, obuf);
            U16Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy42> for Copy42Fmt {
        fn prepare(&self, v: &Copy42) -> Result<usize, PreSerializeError> {
            reveal(<Copy42Fmt as SpecByteLen>::byte_len);
            reveal(<Copy42 as DeepView>::deep_view);
            reveal(Copy42Spec::into_structural);
            let Copy42 {
                offset,
                length,
            } = v;
            let l1 = (U32Be).prepare (offset) ?;
            let l2 = (U16Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy44Fmt {
        type PT = Copy44;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy44Fmt as SpecParser>::spec_parse);
            reveal(<Copy44 as DeepView>::deep_view);
            reveal(Copy44Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy44 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy44> for Copy44Fmt {
        fn serialize_into(&self, v: &Copy44, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy44Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy44Fmt as SpecByteLen>::byte_len);
            reveal(<Copy44 as DeepView>::deep_view);
            reveal(Copy44Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy44 {
                offset,
                length,
            } = v;
            U32Be.serialize_into(offset, obuf);
            U32Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy44> for Copy44Fmt {
        fn prepare(&self, v: &Copy44) -> Result<usize, PreSerializeError> {
            reveal(<Copy44Fmt as SpecByteLen>::byte_len);
            reveal(<Copy44 as DeepView>::deep_view);
            reveal(Copy44Spec::into_structural);
            let Copy44 {
                offset,
                length,
            } = v;
            let l1 = (U32Be).prepare (offset) ?;
            let l2 = (U32Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy48Fmt {
        type PT = Copy48;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy48Fmt as SpecParser>::spec_parse);
            reveal(<Copy48 as DeepView>::deep_view);
            reveal(Copy48Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy48 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy48> for Copy48Fmt {
        fn serialize_into(&self, v: &Copy48, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy48Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy48Fmt as SpecByteLen>::byte_len);
            reveal(<Copy48 as DeepView>::deep_view);
            reveal(Copy48Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy48 {
                offset,
                length,
            } = v;
            U32Be.serialize_into(offset, obuf);
            U64Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy48> for Copy48Fmt {
        fn prepare(&self, v: &Copy48) -> Result<usize, PreSerializeError> {
            reveal(<Copy48Fmt as SpecByteLen>::byte_len);
            reveal(<Copy48 as DeepView>::deep_view);
            reveal(Copy48Spec::into_structural);
            let Copy48 {
                offset,
                length,
            } = v;
            let l1 = (U32Be).prepare (offset) ?;
            let l2 = (U64Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy81Fmt {
        type PT = Copy81;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy81Fmt as SpecParser>::spec_parse);
            reveal(<Copy81 as DeepView>::deep_view);
            reveal(Copy81Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U8).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy81 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy81> for Copy81Fmt {
        fn serialize_into(&self, v: &Copy81, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy81Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy81Fmt as SpecByteLen>::byte_len);
            reveal(<Copy81 as DeepView>::deep_view);
            reveal(Copy81Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy81 {
                offset,
                length,
            } = v;
            U64Be.serialize_into(offset, obuf);
            U8.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy81> for Copy81Fmt {
        fn prepare(&self, v: &Copy81) -> Result<usize, PreSerializeError> {
            reveal(<Copy81Fmt as SpecByteLen>::byte_len);
            reveal(<Copy81 as DeepView>::deep_view);
            reveal(Copy81Spec::into_structural);
            let Copy81 {
                offset,
                length,
            } = v;
            let l1 = (U64Be).prepare (offset) ?;
            let l2 = (U8).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy82Fmt {
        type PT = Copy82;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy82Fmt as SpecParser>::spec_parse);
            reveal(<Copy82 as DeepView>::deep_view);
            reveal(Copy82Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U16Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy82 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy82> for Copy82Fmt {
        fn serialize_into(&self, v: &Copy82, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy82Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy82Fmt as SpecByteLen>::byte_len);
            reveal(<Copy82 as DeepView>::deep_view);
            reveal(Copy82Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy82 {
                offset,
                length,
            } = v;
            U64Be.serialize_into(offset, obuf);
            U16Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy82> for Copy82Fmt {
        fn prepare(&self, v: &Copy82) -> Result<usize, PreSerializeError> {
            reveal(<Copy82Fmt as SpecByteLen>::byte_len);
            reveal(<Copy82 as DeepView>::deep_view);
            reveal(Copy82Spec::into_structural);
            let Copy82 {
                offset,
                length,
            } = v;
            let l1 = (U64Be).prepare (offset) ?;
            let l2 = (U16Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy84Fmt {
        type PT = Copy84;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy84Fmt as SpecParser>::spec_parse);
            reveal(<Copy84 as DeepView>::deep_view);
            reveal(Copy84Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U32Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy84 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy84> for Copy84Fmt {
        fn serialize_into(&self, v: &Copy84, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy84Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy84Fmt as SpecByteLen>::byte_len);
            reveal(<Copy84 as DeepView>::deep_view);
            reveal(Copy84Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy84 {
                offset,
                length,
            } = v;
            U64Be.serialize_into(offset, obuf);
            U32Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy84> for Copy84Fmt {
        fn prepare(&self, v: &Copy84) -> Result<usize, PreSerializeError> {
            reveal(<Copy84Fmt as SpecByteLen>::byte_len);
            reveal(<Copy84 as DeepView>::deep_view);
            reveal(Copy84Spec::into_structural);
            let Copy84 {
                offset,
                length,
            } = v;
            let l1 = (U64Be).prepare (offset) ?;
            let l2 = (U32Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for Copy88Fmt {
        type PT = Copy88;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<Copy88Fmt as SpecParser>::spec_parse);
            reveal(<Copy88 as DeepView>::deep_view);
            reveal(Copy88Spec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, offset) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, length) = (U64Be).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Copy88 {
                offset,
                length,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Copy88> for Copy88Fmt {
        fn serialize_into(&self, v: &Copy88, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<Copy88Fmt as SpecSerializer>::spec_serialize);
            reveal(<Copy88Fmt as SpecByteLen>::byte_len);
            reveal(<Copy88 as DeepView>::deep_view);
            reveal(Copy88Spec::into_structural);
            let ghost old_obuf = obuf@;

            let Copy88 {
                offset,
                length,
            } = v;
            U64Be.serialize_into(offset, obuf);
            U64Be.serialize_into(length, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Copy88> for Copy88Fmt {
        fn prepare(&self, v: &Copy88) -> Result<usize, PreSerializeError> {
            reveal(<Copy88Fmt as SpecByteLen>::byte_len);
            reveal(<Copy88 as DeepView>::deep_view);
            reveal(Copy88Spec::into_structural);
            let Copy88 {
                offset,
                length,
            } = v;
            let l1 = (U64Be).prepare (offset) ?;
            let l2 = (U64Be).prepare (length) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for CommandFmt {
        type PT = Command<'i>;

        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            broadcast use vest_lib::core::spec::SafeParser::lemma_parse_safe;
            broadcast use vest_lib::core::spec::SoundParser::lemma_parse_sound_value;

            reveal(<CommandFmt as SpecParser>::spec_parse);
            reveal(<Command as DeepView>::deep_view);
            reveal(CommandSpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            let (n1, opcode) = (U8).parse (& rest) ?;
            let rest = rest.skip(n1);
            let (n2, body) = (Named ("command_body", CommandBodyFmt {
                opcode: opcode
            }
            )).parse (& rest) ?;
            let rest = rest.skip(n2);
            let total_n = n1 + n2;
            let final_v = Command {
                opcode,
                body,
            };
            assert(self.spec_parse(ibuf@) == Some((total_n as int, final_v.deep_view())));
            Ok((total_n, final_v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, Command<'i>> for CommandFmt {
        fn serialize_into(&self, v: &Command<'i>, obuf: &mut Output) {
            broadcast use vest_lib::core::exec::output::outbuf_lemmas;
            reveal(<CommandFmt as SpecSerializer>::spec_serialize);
            reveal(<CommandFmt as SpecByteLen>::byte_len);
            reveal(<Command as DeepView>::deep_view);
            reveal(CommandSpec::into_structural);
            let ghost old_obuf = obuf@;

            let Command {
                opcode,
                body,
            } = v;
            U8.serialize_into(opcode, obuf);
            CommandBodyFmt {
                opcode: * opcode
            }
            .serialize_into(body, obuf);

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<Command<'i>> for CommandFmt {
        fn prepare(&self, v: &Command<'i>) -> Result<usize, PreSerializeError> {
            reveal(<CommandFmt as SpecByteLen>::byte_len);
            reveal(<Command as DeepView>::deep_view);
            reveal(CommandSpec::into_structural);
            let Command {
                opcode,
                body,
            } = v;
            let l1 = (U8).prepare (opcode) ?;
            let l2 = (Named ("command_body", CommandBodyFmt {
                opcode: * opcode
            }
            )).prepare (body) ?;
            let total_len = l1.checked_add (l2).ok_or (PreSerializeError::length_too_large()) ?;
            Ok(total_len)
        }
    }



    impl<'i> Parser<&'i [u8]> for CommandBodyFmt {
        type PT = CommandBody<'i>;

        #[verifier::spinoff_prover]
        fn parse(&self, ibuf: &&'i [u8]) -> PResult<Self::PT> {
            reveal(<CommandBodyFmt as SpecParser>::spec_parse);
            reveal(<CommandBody as DeepView>::deep_view);
            reveal(CommandBodySpec::from_structural);
            let _ = ibuf.len();
            let rest = *ibuf;

            proof {
                use_type_invariant(self);
            }

            let (n, v) = match self.opcode {
                0 => {
                    let (n,
                    v) = (Empty).parse (& rest) ?;
                    (n,
                    CommandBody::Variant1 (v))
                }
                ,
                1 ..= 64 => {
                    let (n,
                    v) = (Varied (self.opcode)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant2 (v))
                }
                ,
                65 => {
                    let (n,
                    v) = (Named ("literal1",
                    Literal1Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant3 (v))
                }
                ,
                66 => {
                    let (n,
                    v) = (Named ("literal2",
                    Literal2Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant4 (v))
                }
                ,
                67 => {
                    let (n,
                    v) = (Named ("literal4",
                    Literal4Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant5 (v))
                }
                ,
                68 => {
                    let (n,
                    v) = (Named ("literal8",
                    Literal8Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant6 (v))
                }
                ,
                69 => {
                    let (n,
                    v) = (Named ("copy1_1",
                    Copy11Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant7 (v))
                }
                ,
                70 => {
                    let (n,
                    v) = (Named ("copy1_2",
                    Copy12Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant8 (v))
                }
                ,
                71 => {
                    let (n,
                    v) = (Named ("copy1_4",
                    Copy14Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant9 (v))
                }
                ,
                72 => {
                    let (n,
                    v) = (Named ("copy1_8",
                    Copy18Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant10 (v))
                }
                ,
                73 => {
                    let (n,
                    v) = (Named ("copy2_1",
                    Copy21Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant11 (v))
                }
                ,
                74 => {
                    let (n,
                    v) = (Named ("copy2_2",
                    Copy22Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant12 (v))
                }
                ,
                75 => {
                    let (n,
                    v) = (Named ("copy2_4",
                    Copy24Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant13 (v))
                }
                ,
                76 => {
                    let (n,
                    v) = (Named ("copy2_8",
                    Copy28Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant14 (v))
                }
                ,
                77 => {
                    let (n,
                    v) = (Named ("copy4_1",
                    Copy41Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant15 (v))
                }
                ,
                78 => {
                    let (n,
                    v) = (Named ("copy4_2",
                    Copy42Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant16 (v))
                }
                ,
                79 => {
                    let (n,
                    v) = (Named ("copy4_4",
                    Copy44Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant17 (v))
                }
                ,
                80 => {
                    let (n,
                    v) = (Named ("copy4_8",
                    Copy48Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant18 (v))
                }
                ,
                81 => {
                    let (n,
                    v) = (Named ("copy8_1",
                    Copy81Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant19 (v))
                }
                ,
                82 => {
                    let (n,
                    v) = (Named ("copy8_2",
                    Copy82Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant20 (v))
                }
                ,
                83 => {
                    let (n,
                    v) = (Named ("copy8_4",
                    Copy84Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant21 (v))
                }
                ,
                84 => {
                    let (n,
                    v) = (Named ("copy8_8",
                    Copy88Fmt)).parse (& rest) ?;
                    (n,
                    CommandBody::Variant22 (v))
                }
                ,
                _ => {
                    let (n,
                    v) = (Void ("reserved librsync delta opcode")).parse (& rest) ?;
                    (n,
                    CommandBody::Default (v))
                }
                ,
            };
            assert(self.spec_parse(ibuf@) == Some((n as int, v.deep_view())));
            Ok((n, v))
        }
    }

    impl<Output: OutputBuf, 'i> Serializer<Output, CommandBody<'i>> for CommandBodyFmt {
        #[verifier::spinoff_prover]
        fn serialize_into(&self, v: &CommandBody<'i>, obuf: &mut Output) {
            reveal(<CommandBodyFmt as SpecSerializer>::spec_serialize);
            reveal(<CommandBodyFmt as SpecByteLen>::byte_len);
            reveal(<CommandBody as DeepView>::deep_view);
            reveal(CommandBodySpec::into_structural);
            proof {
                use_type_invariant(self);
            }

            let ghost old_obuf = obuf@;

            match (self.opcode, v) {
                (0, CommandBody::Variant1 (v)) => {
                    (Empty).serialize_into (v,
                    obuf) ;
                }
                ,
                (x, CommandBody::Variant2 (v)) if x >= 1 && x <= 64 => {
                    (Varied (self.opcode)).serialize_into (*v,
                    obuf) ;
                }
                ,
                (65, CommandBody::Variant3 (v)) => {
                    (Literal1Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (66, CommandBody::Variant4 (v)) => {
                    (Literal2Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (67, CommandBody::Variant5 (v)) => {
                    (Literal4Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (68, CommandBody::Variant6 (v)) => {
                    (Literal8Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (69, CommandBody::Variant7 (v)) => {
                    (Copy11Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (70, CommandBody::Variant8 (v)) => {
                    (Copy12Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (71, CommandBody::Variant9 (v)) => {
                    (Copy14Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (72, CommandBody::Variant10 (v)) => {
                    (Copy18Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (73, CommandBody::Variant11 (v)) => {
                    (Copy21Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (74, CommandBody::Variant12 (v)) => {
                    (Copy22Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (75, CommandBody::Variant13 (v)) => {
                    (Copy24Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (76, CommandBody::Variant14 (v)) => {
                    (Copy28Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (77, CommandBody::Variant15 (v)) => {
                    (Copy41Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (78, CommandBody::Variant16 (v)) => {
                    (Copy42Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (79, CommandBody::Variant17 (v)) => {
                    (Copy44Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (80, CommandBody::Variant18 (v)) => {
                    (Copy48Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (81, CommandBody::Variant19 (v)) => {
                    (Copy81Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (82, CommandBody::Variant20 (v)) => {
                    (Copy82Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (83, CommandBody::Variant21 (v)) => {
                    (Copy84Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (84, CommandBody::Variant22 (v)) => {
                    (Copy88Fmt).serialize_into (v,
                    obuf) ;
                }
                ,
                (_, CommandBody::Default (v)) => {
                    (Void ("reserved librsync delta opcode")).serialize_into (v,
                    obuf) ;
                }
                ,
                _ => {},
            }

            assert(obuf@ == old_obuf + self.spec_serialize(v.deep_view()));
        }
    }

    impl<'i> Prepare<CommandBody<'i>> for CommandBodyFmt {
        #[verifier::spinoff_prover]
        fn prepare(&self, v: &CommandBody<'i>) -> Result<usize, PreSerializeError> {
            reveal(<CommandBodyFmt as SpecByteLen>::byte_len);
            reveal(<CommandBody as DeepView>::deep_view);
            reveal(CommandBodySpec::into_structural);
            proof {
                use_type_invariant(self);
            }

            match (self.opcode, v) {
                (0, CommandBody::Variant1 (v)) => (Empty).prepare (v),
                (x, CommandBody::Variant2 (v)) if x >= 1 && x <= 64 => (Varied (self.opcode)).prepare (v),
                (65, CommandBody::Variant3 (v)) => (Named ("literal1", Literal1Fmt)).prepare (v),
                (66, CommandBody::Variant4 (v)) => (Named ("literal2", Literal2Fmt)).prepare (v),
                (67, CommandBody::Variant5 (v)) => (Named ("literal4", Literal4Fmt)).prepare (v),
                (68, CommandBody::Variant6 (v)) => (Named ("literal8", Literal8Fmt)).prepare (v),
                (69, CommandBody::Variant7 (v)) => (Named ("copy1_1", Copy11Fmt)).prepare (v),
                (70, CommandBody::Variant8 (v)) => (Named ("copy1_2", Copy12Fmt)).prepare (v),
                (71, CommandBody::Variant9 (v)) => (Named ("copy1_4", Copy14Fmt)).prepare (v),
                (72, CommandBody::Variant10 (v)) => (Named ("copy1_8", Copy18Fmt)).prepare (v),
                (73, CommandBody::Variant11 (v)) => (Named ("copy2_1", Copy21Fmt)).prepare (v),
                (74, CommandBody::Variant12 (v)) => (Named ("copy2_2", Copy22Fmt)).prepare (v),
                (75, CommandBody::Variant13 (v)) => (Named ("copy2_4", Copy24Fmt)).prepare (v),
                (76, CommandBody::Variant14 (v)) => (Named ("copy2_8", Copy28Fmt)).prepare (v),
                (77, CommandBody::Variant15 (v)) => (Named ("copy4_1", Copy41Fmt)).prepare (v),
                (78, CommandBody::Variant16 (v)) => (Named ("copy4_2", Copy42Fmt)).prepare (v),
                (79, CommandBody::Variant17 (v)) => (Named ("copy4_4", Copy44Fmt)).prepare (v),
                (80, CommandBody::Variant18 (v)) => (Named ("copy4_8", Copy48Fmt)).prepare (v),
                (81, CommandBody::Variant19 (v)) => (Named ("copy8_1", Copy81Fmt)).prepare (v),
                (82, CommandBody::Variant20 (v)) => (Named ("copy8_2", Copy82Fmt)).prepare (v),
                (83, CommandBody::Variant21 (v)) => (Named ("copy8_4", Copy84Fmt)).prepare (v),
                (84, CommandBody::Variant22 (v)) => (Named ("copy8_8", Copy88Fmt)).prepare (v),
                (x, CommandBody::Default (v)) if ! (x == 0) && ! (x >= 1 && x <= 64) && ! (x == 65) && ! (x == 66) && ! (x == 67) && ! (x == 68) && ! (x == 69) && ! (x == 70) && ! (x == 71) && ! (x == 72) && ! (x == 73) && ! (x == 74) && ! (x == 75) && ! (x == 76) && ! (x == 77) && ! (x == 78) && ! (x == 79) && ! (x == 80) && ! (x == 81) && ! (x == 82) && ! (x == 83) && ! (x == 84) => (Void ("reserved librsync delta opcode")).prepare (v),
                 _ => Err(PreSerializeError::not_compliant(ComplianceErrorKind::InvalidTag)),
            }
        }
    }

}
}

include!("../verified/delta_refinement.rs");
