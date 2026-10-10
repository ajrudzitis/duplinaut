// Exact public tar-header byte layout, independently stated from DSL framing.
verus! {
pub open spec fn tar_header_wire(input: Seq<u8>) -> Option<TarHeaderSpec> {
    if input.len() < 512 { None } else { Some(TarHeaderSpec {
        name: input.subrange(0, 100),
        mode: input.subrange(100, 108),
        uid: input.subrange(108, 116),
        gid: input.subrange(116, 124),
        size: input.subrange(124, 136),
        mtime: input.subrange(136, 148),
        checksum: input.subrange(148, 156),
        entry_type: input[156],
        linkname: input.subrange(157, 257),
        magic_version: input.subrange(257, 265),
        uname: input.subrange(265, 297),
        gname: input.subrange(297, 329),
        device_major: input.subrange(329, 337),
        device_minor: input.subrange(337, 345),
        prefix: input.subrange(345, 500),
        unused: input.subrange(500, 512),
    }) }
}
pub proof fn lemma_tar_header_refinement(input: Seq<u8>)
    ensures match TarHeaderFmt.spec_parse(input) {
        Some((consumed, header)) => consumed == 512 && tar_header_wire(input) == Some(header),
        None => tar_header_wire(input) is None,
    },
{
    reveal(<TarHeaderFmt as SpecParser>::spec_parse);
    reveal(TarHeaderSpec::from_structural);
}
}
