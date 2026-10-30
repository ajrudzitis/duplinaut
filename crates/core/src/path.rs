//! Byte identities for paths in a backup. Rendered terminal text is not identity.
// Explicit length predicates are shared with proof guards.
#![allow(clippy::len_zero)]
use vstd::prelude::*;

verus! {
/// Lexicographic byte order, including the shorter-prefix rule.
pub open spec fn bytes_less(left: Seq<u8>, right: Seq<u8>) -> bool {
    (left.len() < right.len() && left == right.take(left.len() as int))
    || exists|i: int| 0 <= i < left.len() && i < right.len()
        && left.take(i) == right.take(i) && left[i] < right[i]
}

/// Lexicographic order on components, rather than on slash-joined rendering.
pub open spec fn components_less(left: Seq<Seq<u8>>, right: Seq<Seq<u8>>) -> bool {
    (left.len() < right.len() && left == right.take(left.len() as int))
    || exists|i: int| 0 <= i < left.len() && i < right.len()
        && left.take(i) == right.take(i) && bytes_less(left[i], right[i])
}

proof fn bytes_less_total_from(left: Seq<u8>, right: Seq<u8>, index: int)
    requires
        0 <= index <= left.len(),
        index <= right.len(),
        left.take(index) == right.take(index),
        left != right,
    ensures bytes_less(left, right) || bytes_less(right, left),
    decreases (if left.len() < right.len() { left.len() } else { right.len() }) - index,
{
    let limit = if left.len() < right.len() { left.len() } else { right.len() };
    if index < limit {
        if left[index] == right[index] {
            assert(left.take(index + 1) == right.take(index + 1)) by {
                assert forall|i: int| 0 <= i < index + 1
                    implies left[i] == right[i] by {
                    if i < index {
                        assert(left.take(index)[i] == left[i]);
                        assert(left.take(index)[i] == right.take(index)[i]);
                        assert(right.take(index)[i] == right[i]);
                    } else {
                        assert(i == index);
                    }
                }
            }
            bytes_less_total_from(left, right, index + 1);
        } else if left[index] < right[index] {
            assert(bytes_less(left, right)) by {
                reveal(bytes_less);
                assert(exists|i: int| 0 <= i < left.len() && i < right.len()
                    && left.take(i) == right.take(i) && left[i] < right[i]) by {
                    assert(left.take(index) == right.take(index));
                }
            }
        } else {
            assert(right[index] < left[index]);
            assert(bytes_less(right, left)) by {
                reveal(bytes_less);
                assert(exists|i: int| 0 <= i < right.len() && i < left.len()
                    && right.take(i) == left.take(i) && right[i] < left[i]) by {
                    assert(right.take(index) == left.take(index));
                }
            }
        }
    } else if left.len() < right.len() {
        assert(index == left.len());
        assert(left == right.take(left.len() as int)) by {
            assert forall|i: int| 0 <= i < left.len()
                implies left[i] == right.take(left.len() as int)[i] by {
                assert(left.take(index)[i] == left[i]);
                assert(left.take(index)[i] == right.take(index)[i]);
                assert(right.take(index)[i] == right[i]);
            }
        }
        assert(bytes_less(left, right));
    } else if right.len() < left.len() {
        assert(index == right.len());
        assert(right == left.take(right.len() as int)) by {
            assert forall|i: int| 0 <= i < right.len()
                implies right[i] == left.take(right.len() as int)[i] by {
                assert(left.take(index)[i] == left[i]);
                assert(left.take(index)[i] == right.take(index)[i]);
                assert(right.take(index)[i] == right[i]);
            }
        }
        assert(bytes_less(right, left));
    } else {
        assert(index == left.len() && index == right.len());
        assert(left == right) by {
            assert forall|i: int| 0 <= i < left.len()
                implies left[i] == right[i] by {
                assert(left.take(index)[i] == left[i]);
                assert(left.take(index)[i] == right.take(index)[i]);
                assert(right.take(index)[i] == right[i]);
            }
        }
        assert(false);
    }
}

proof fn bytes_less_total(left: Seq<u8>, right: Seq<u8>)
    requires left != right,
    ensures bytes_less(left, right) || bytes_less(right, left),
{
    assert(left.take(0) == right.take(0));
    bytes_less_total_from(left, right, 0);
}

proof fn bytes_less_is_asymmetric(left: Seq<u8>, right: Seq<u8>)
    requires bytes_less(left, right),
    ensures !bytes_less(right, left),
{
    reveal(bytes_less);
    if left.len() < right.len() && left == right.take(left.len() as int) {
        assert(!bytes_less(right, left)) by {
            reveal(bytes_less);
            assert(!(right.len() < left.len()));
            assert forall|i: int| 0 <= i < right.len() && i < left.len()
                implies !(right.take(i) == left.take(i) && right[i] < left[i]) by {
                assert(right[i] == left[i]);
            }
        }
    } else {
        assert(exists|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && left[i] < right[i]);
        let first = choose|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && left[i] < right[i];
        assert(!bytes_less(right, left)) by {
            reveal(bytes_less);
            assert(!(right.len() < left.len()
                && right == left.take(right.len() as int))) by {
                if right.len() < left.len() && right == left.take(right.len() as int) {
                    assert(left.take(right.len() as int)[first] == left[first]);
                    assert(left.take(right.len() as int)[first]
                        == right.take(right.len() as int)[first]);
                    assert(right.take(right.len() as int)[first] == right[first]);
                    assert(right[first] == left[first]);
                    assert(left[first] < right[first]);
                }
            }
            assert forall|i: int| 0 <= i < right.len() && i < left.len()
                implies !(right.take(i) == left.take(i) && right[i] < left[i]) by {
                if i < first {
                    assert(left.take(first)[i] == left[i]);
                    assert(left.take(first)[i] == right.take(first)[i]);
                    assert(right.take(first)[i] == right[i]);
                    assert(left[i] == right[i]);
                } else if i == first {
                    assert(left[i] < right[i]);
                } else {
                    assert(left.take(i) != right.take(i)) by {
                        assert(left.take(i)[first] == left[first]);
                        assert(right.take(i)[first] == right[first]);
                    }
                }
            }
        }
    }
}

proof fn components_less_is_asymmetric(left: Seq<Seq<u8>>, right: Seq<Seq<u8>>)
    requires components_less(left, right),
    ensures !components_less(right, left),
{
    reveal(components_less);
    if left.len() < right.len() && left == right.take(left.len() as int) {
        assert(!components_less(right, left)) by {
            reveal(components_less);
            assert(!(right.len() < left.len()));
            assert forall|i: int| 0 <= i < right.len() && i < left.len()
                implies !(right.take(i) == left.take(i) && bytes_less(right[i], left[i])) by {
                assert(right[i] == left[i]);
            }
        }
    } else {
        assert(exists|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && bytes_less(left[i], right[i]));
        let first = choose|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && bytes_less(left[i], right[i]);
        bytes_less_is_asymmetric(left[first], right[first]);
        assert(!components_less(right, left)) by {
            reveal(components_less);
            assert(!(right.len() < left.len()
                && right == left.take(right.len() as int))) by {
                if right.len() < left.len() && right == left.take(right.len() as int) {
                    assert(left.take(right.len() as int)[first] == left[first]);
                    assert(left.take(right.len() as int)[first]
                        == right.take(right.len() as int)[first]);
                    assert(right.take(right.len() as int)[first] == right[first]);
                    assert(left[first] != right[first]);
                }
            }
            assert forall|i: int| 0 <= i < right.len() && i < left.len()
                implies !(right.take(i) == left.take(i) && bytes_less(right[i], left[i])) by {
                if i < first {
                    assert(left.take(first)[i] == left[i]);
                    assert(left.take(first)[i] == right.take(first)[i]);
                    assert(right.take(first)[i] == right[i]);
                    assert(left[i] == right[i]);
                } else if i == first {
                    assert(!bytes_less(right[i], left[i]));
                } else {
                    assert(left.take(i) != right.take(i)) by {
                        assert(left.take(i)[first] == left[first]);
                        assert(right.take(i)[first] == right[first]);
                    }
                }
            }
        }
    }
}

/// True when `prefix` is the same component sequence as `path` or an ancestor.
/// Component equality preserves boundaries: `a` is not a prefix of `a-`.
pub open spec fn component_prefix(prefix: Seq<Seq<u8>>, path: Seq<Seq<u8>>) -> bool {
    prefix.len() <= path.len()
        && forall|i: int| 0 <= i < prefix.len() ==> prefix[i] == path[i]
}

/// Paths sharing a component prefix form one interval in component order.
//= docs/parsing-requirements.md#parse-state-001
//= type=implementation
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
pub proof fn component_prefix_is_order_convex(
    prefix: Seq<Seq<u8>>,
    left: Seq<Seq<u8>>,
    middle: Seq<Seq<u8>>,
    right: Seq<Seq<u8>>,
)
    requires
        component_prefix(prefix, left),
        component_prefix(prefix, right),
        components_less(left, middle),
        components_less(middle, right),
    ensures component_prefix(prefix, middle),
{
    reveal(component_prefix);
    reveal(components_less);
    assert(prefix.take(0) == middle.take(0));
    component_prefix_is_order_convex_from(prefix, left, middle, right, 0);
}

proof fn component_prefix_is_order_convex_from(
    prefix: Seq<Seq<u8>>,
    left: Seq<Seq<u8>>,
    middle: Seq<Seq<u8>>,
    right: Seq<Seq<u8>>,
    index: int,
)
    requires
        component_prefix(prefix, left),
        component_prefix(prefix, right),
        components_less(left, middle),
        components_less(middle, right),
        0 <= index,
        index <= prefix.len(),
        index <= middle.len(),
        prefix.take(index) == middle.take(index),
    ensures component_prefix(prefix, middle),
    decreases (if prefix.len() < middle.len() { prefix.len() } else { middle.len() }) - index,
{
    reveal(component_prefix);
    reveal(components_less);
    let limit = if prefix.len() < middle.len() { prefix.len() } else { middle.len() };
    if index < limit {
        if prefix[index] == middle[index] {
            assert(prefix.take(index + 1) == middle.take(index + 1)) by {
                assert forall|i: int| 0 <= i < index + 1
                    implies prefix[i] == middle[i] by {
                    if i < index {
                        assert(prefix.take(index)[i] == prefix[i]);
                        assert(prefix.take(index)[i] == middle.take(index)[i]);
                        assert(middle.take(index)[i] == middle[i]);
                    } else {
                        assert(i == index);
                    }
                }
            }
            component_prefix_is_order_convex_from(prefix, left, middle, right, index + 1);
        } else {
            bytes_less_total(prefix[index], middle[index]);
            if bytes_less(prefix[index], middle[index]) {
                assert(components_less(right, middle)) by {
                    reveal(components_less);
                    assert(exists|i: int| 0 <= i < right.len() && i < middle.len()
                        && right.take(i) == middle.take(i)
                        && bytes_less(right[i], middle[i])) by {
                        assert(right.take(index) == middle.take(index)) by {
                            assert(right.take(index) == prefix.take(index)) by {
                                assert forall|j: int| 0 <= j < index
                                    implies right[j] == prefix[j] by {
                                    assert(j < prefix.len());
                                    assert(prefix[j] == right[j]);
                                }
                            }
                            assert(prefix.take(index) == middle.take(index));
                        }
                        assert(bytes_less(right[index], middle[index])) by {
                            assert(right[index] == prefix[index]);
                        }
                    }
                }
                components_less_is_asymmetric(middle, right);
                assert(false);
            } else {
                assert(bytes_less(middle[index], prefix[index]));
                assert(components_less(middle, left)) by {
                    reveal(components_less);
                    assert(exists|i: int| 0 <= i < middle.len() && i < left.len()
                        && middle.take(i) == left.take(i)
                        && bytes_less(middle[i], left[i])) by {
                        assert(middle.take(index) == left.take(index)) by {
                            assert(middle.take(index) == prefix.take(index));
                            assert(prefix.take(index) == left.take(index)) by {
                                assert forall|j: int| 0 <= j < index
                                    implies prefix[j] == left[j] by {
                                    assert(j < prefix.len());
                                    assert(prefix[j] == left[j]);
                                }
                            }
                        }
                        assert(bytes_less(middle[index], left[index])) by {
                            assert(left[index] == prefix[index]);
                        }
                    }
                }
                components_less_is_asymmetric(left, middle);
                assert(false);
            }
        }
    } else if prefix.len() <= middle.len() {
        assert(index == prefix.len());
        assert forall|i: int| 0 <= i < prefix.len()
            implies prefix[i] == middle[i] by {
            assert(prefix.take(index)[i] == prefix[i]);
            assert(prefix.take(index)[i] == middle.take(index)[i]);
            assert(middle.take(index)[i] == middle[i]);
        }
    } else {
        assert(index == middle.len());
        assert(middle == left.take(middle.len() as int)) by {
            assert forall|i: int| 0 <= i < middle.len()
                implies middle[i] == left.take(middle.len() as int)[i] by {
                assert(middle.take(index)[i] == middle[i]);
                assert(middle.take(index)[i] == prefix.take(index)[i]);
                assert(prefix.take(index)[i] == prefix[i]);
                assert(prefix[i] == left[i]);
                assert(left.take(middle.len() as int)[i] == left[i]);
            }
        }
        assert(components_less(middle, left));
        components_less_is_asymmetric(middle, left);
        assert(false);
    }
}

pub open spec fn valid_component(component: Seq<u8>) -> bool {
    component.len() > 0 && component != seq![46u8] && component != seq![46u8, 46u8]
        && forall|i: int| 0 <= i < component.len() ==> component[i] != 0 && component[i] != 47
}

pub open spec fn root_marker(bytes: Seq<u8>, directory_suffix: bool) -> bool {
    bytes.len() == 0 || bytes == seq![46u8] || (directory_suffix && bytes == seq![46u8, 47u8])
}

/// Functional byte grammar: finish a component at a slash or the normalized
/// end, preserving diagnostic precedence and exact component slices.
pub open spec fn scan_path(bytes: Seq<u8>, start: int, cursor: int, end: int,
    max_components: int, components: Seq<Seq<u8>>) -> Result<Seq<Seq<u8>>, PathError>
    recommends 0 <= start <= cursor <= end <= bytes.len(),
    decreases end - cursor,
{
    if !(0 <= start <= cursor <= end <= bytes.len()) { Err(PathError::EmptyComponent) }
    else if cursor == end || bytes[cursor] == 47 {
        let component = bytes.subrange(start, cursor);
        if cursor == start { Err(PathError::EmptyComponent) }
        else if component == seq![46u8] { Err(PathError::DotComponent) }
        else if component == seq![46u8, 46u8] { Err(PathError::ParentComponent) }
        else if components.len() == max_components { Err(PathError::ComponentLimit) }
        else if cursor == end { Ok(components.push(component)) }
        else { scan_path(bytes, cursor + 1, cursor + 1, end, max_components, components.push(component)) }
    } else if bytes[cursor] == 0 { Err(PathError::Nul) }
    else { scan_path(bytes, start, cursor + 1, end, max_components, components) }
}

pub open spec fn path_model(bytes: Seq<u8>, directory_suffix: bool,
    max_bytes: int, max_components: int) -> Result<Seq<Seq<u8>>, PathError> {
    if bytes.len() > max_bytes { Err(PathError::ByteLimit) }
    else if root_marker(bytes, directory_suffix) { Ok(seq![]) }
    else if bytes[0] == 47 { Err(PathError::Absolute) }
    else {
        let end = if directory_suffix && bytes[bytes.len() as int - 1] == 47 { bytes.len() as int - 1 }
            else { bytes.len() as int };
        scan_path(bytes, 0, 0, end, max_components, seq![])
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BackupPath { components: Vec<Vec<u8>> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError { Absolute, EmptyComponent, DotComponent, ParentComponent, Nul, ByteLimit, ComponentLimit }

impl BackupPath {
    pub closed spec fn view(&self) -> Seq<Seq<u8>> {
        self.components@.map_values(|component: Vec<u8>| component@)
    }
    /// Root markers are decoded here; one trailing separator is allowed only
    /// when the container explicitly identifies a directory.
    //= docs/parsing-requirements.md#parse-path-001
    //# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
    //= docs/parsing-requirements.md#parse-path-002
    //# A restore path MUST be relative to the backup root and contain no NUL, empty interior component, dot component, or parent component after format-specific root decoding.
    pub fn parse(bytes: &[u8], directory_suffix: bool, max_bytes: usize, max_components: usize) -> (result: Result<Self, PathError>)
        ensures
            match result { Ok(path) => path_model(bytes@, directory_suffix, max_bytes as int, max_components as int) == Ok(path@),
                Err(error) => path_model(bytes@, directory_suffix, max_bytes as int, max_components as int) == Err(error), },
            match result {
            Ok(path) => bytes.len() <= max_bytes && path@.len() <= max_components
                && (path@.len() == 0 <==> root_marker(bytes@, directory_suffix))
                && forall|i: int| 0 <= i < path@.len() ==> valid_component(#[trigger] path@[i]),
            Err(_) => true,
        },
    {
        reveal(BackupPath::view);
        reveal_with_fuel(path_model, 1);
        if bytes.len() > max_bytes { return Err(PathError::ByteLimit); }
        if bytes.len() == 0 || (bytes.len() == 1 && bytes[0] == b'.')
            || (directory_suffix && bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'/') {
            proof {
                if bytes.len() == 1 { assert(bytes@ =~= seq![46u8]); }
                if bytes.len() == 2 { assert(bytes@ =~= seq![46u8, 47u8]); }
            }
            let components: Vec<Vec<u8>> = Vec::new();
            proof { assert(components@.map_values(|component: Vec<u8>| component@) =~= seq![]); }
            return Ok(Self { components });
        }
        if bytes[0] == b'/' { return Err(PathError::Absolute); }
        let mut end = bytes.len();
        if directory_suffix && bytes[end - 1] == b'/' { end -= 1; }
        let mut start = 0;
        let mut cursor = 0;
        let mut components: Vec<Vec<u8>> = Vec::new();
        proof { assert(components@.map_values(|component: Vec<u8>| component@) =~= seq![]); }
        loop
            invariant_except_break
                path_model(bytes@, directory_suffix, max_bytes as int, max_components as int)
                    == scan_path(bytes@, start as int, cursor as int, end as int, max_components as int,
                        components@.map_values(|component: Vec<u8>| component@)),
            invariant
                0 < end <= bytes.len(),
                bytes.len() <= max_bytes,
                start <= cursor <= end,
                components.len() <= max_components,
                !root_marker(bytes@, directory_suffix),
                forall|i: int| 0 <= i < components.len() ==> valid_component(#[trigger] components@[i]@),
                forall|i: int| start <= i < cursor ==> bytes@[i] != 0 && bytes@[i] != 47,
            ensures
                components.len() > 0,
                path_model(bytes@, directory_suffix, max_bytes as int, max_components as int)
                    == Ok(components@.map_values(|component: Vec<u8>| component@)),
            decreases end - cursor,
        {
            proof { reveal_with_fuel(scan_path, 1); }
            if cursor == end || bytes[cursor] == b'/' {
                let count = cursor - start;
                if count == 0 { return Err(PathError::EmptyComponent); }
                proof {
                    assert(bytes@.subrange(start as int, cursor as int).len() == count);
                    assert(bytes@.subrange(start as int, cursor as int)[0] == bytes@[start as int]);
                    if count == 1 && bytes@[start as int] == b'.' {
                        assert(bytes@.subrange(start as int, cursor as int) =~= seq![46u8]);
                    }
                    if count == 2 && bytes@[start as int] == b'.' && bytes@[start as int + 1] == b'.' {
                        assert(bytes@.subrange(start as int, cursor as int) =~= seq![46u8, 46u8]);
                    }
                    if count != 1 || bytes@[start as int] != b'.' {
                        assert(bytes@.subrange(start as int, cursor as int) != seq![46u8]);
                    }
                    if count != 2 || bytes@[start as int] != b'.' || bytes@[start as int + 1] != b'.' {
                        assert(bytes@.subrange(start as int, cursor as int) != seq![46u8, 46u8]);
                    }
                }
                if count == 1 && bytes[start] == b'.' { return Err(PathError::DotComponent); }
                if count == 2 && bytes[start] == b'.' && bytes[start + 1] == b'.' {
                    return Err(PathError::ParentComponent);
                }
                if components.len() == max_components { return Err(PathError::ComponentLimit); }
                let mut component: Vec<u8> = Vec::new();
                let mut index = start;
                while index < cursor
                    invariant
                        start <= index <= cursor <= end <= bytes.len(),
                        component@ == bytes@.subrange(start as int, index as int),
                        0 < cursor - start,
                        cursor - start == 1 ==> bytes@[start as int] != 46,
                        cursor - start == 2 ==> bytes@[start as int] != 46 || bytes@[start as int + 1] != 46,
                        forall|i: int| start <= i < cursor ==> bytes@[i] != 0 && bytes@[i] != 47,
                        path_model(bytes@, directory_suffix, max_bytes as int, max_components as int)
                            == scan_path(bytes@, start as int, cursor as int, end as int, max_components as int,
                                components@.map_values(|component: Vec<u8>| component@)),
                    decreases cursor - index,
                {
                    component.push(bytes[index]);
                    index += 1;
                }
                proof {
                    assert(component@ =~= bytes@.subrange(start as int, cursor as int));
                    assert(component.len() == cursor - start);
                    assert(component@[0] == bytes@[start as int]);
                    assert forall|i: int| 0 <= i < component@.len() implies component@[i] != 0 && component@[i] != 47 by { }
                    assert(component@ != seq![46u8]) by {
                        if component@ == seq![46u8] { assert(bytes@[start as int] == 46); }
                    }
                    assert(component@ != seq![46u8, 46u8]) by {
                        if component@ == seq![46u8, 46u8] {
                            assert(bytes@[start as int] == 46);
                            assert(bytes@[start as int + 1] == 46);
                        }
                    }
                    assert(valid_component(component@));
                }
                let ghost previous = components@.map_values(|component: Vec<u8>| component@);
                let ghost added = component@;
                components.push(component);
                proof {
                    assert(components@.map_values(|component: Vec<u8>| component@) =~= previous.push(added));
                    reveal_with_fuel(scan_path, 1);
                }
                if cursor == end { break; }
                start = cursor + 1;
            } else if bytes[cursor] == 0 { return Err(PathError::Nul); }
            cursor += 1;
        }
        proof {
            assert(components.len() > 0);
            assert(components.len() <= max_components);
            assert(bytes.len() <= max_bytes);
            assert forall|i: int| 0 <= i < components.len() implies valid_component(#[trigger] components@[i]@) by { }
        }
        Ok(Self { components })
    }

    pub fn components(&self) -> (result: &Vec<Vec<u8>>)
        ensures result@.map_values(|component: Vec<u8>| component@) == self@,
    { &self.components }

    /// Component-wise ancestor test, including equality and the empty root.
    //= docs/parsing-requirements.md#parse-state-001
    //= type=implementation
    //# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
    pub fn is_component_prefix_of(&self, path: &Self) -> (result: bool)
        ensures result == component_prefix(self@, path@),
    {
        reveal(BackupPath::view);
        let mut index = 0;
        while index < self.components.len() && index < path.components.len()
            invariant
                index <= self.components.len(),
                index <= path.components.len(),
                forall|i: int| 0 <= i < index ==> self@[i] == path@[i],
            decreases self.components.len() - index,
        {
            if self.components[index] != path.components[index] {
                proof {
                    reveal(component_prefix);
                    reveal(BackupPath::view);
                    assert(self@[index as int] == self.components[index as int]@);
                    assert(path@[index as int] == path.components[index as int]@);
                    assert(self@[index as int] != path@[index as int]);
                    if self@.len() <= path@.len() {
                        assert(!(forall|i: int| 0 <= i < self@.len()
                            ==> self@[i] == path@[i]));
                    }
                }
                return false;
            }
            index += 1;
        }
        let result = index == self.components.len();
        proof {
            reveal(component_prefix);
            reveal(BackupPath::view);
            if result {
                assert(index == self@.len());
                assert(index <= path@.len());
                assert forall|i: int| 0 <= i < self@.len()
                    implies self@[i] == path@[i] by {
                    assert(i < index);
                }
            } else {
                assert(index == path@.len());
                assert(path@.len() < self@.len());
            }
        }
        result
    }

    /// Compare byte-component sequences, independently of terminal rendering.
    /// Returns -1, 0, or 1. This kernel supplies manifest range comparisons.
    //= docs/parsing-requirements.md#parse-state-003
    //= type=implementation
    //# `SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components. Each component MUST be compared as an unsigned byte sequence using lexicographic byte order, with a proper prefix ordered before the longer sequence. Paths MUST be compared as sequences of components using the same lexicographic rule, with a proper component-sequence prefix ordered before the longer path. Component boundaries MUST remain significant; the ordering MUST NOT be derived by comparing slash-joined or rendered path strings.
    pub fn compare(&self, other: &Self) -> (result: i8)
        ensures
            result == -1 || result == 0 || result == 1,
            result == -1 ==> components_less(self@, other@),
            result == 0 ==> self@ == other@,
            result == 1 ==> components_less(other@, self@),
            components_less(self@, other@) ==> result == -1,
            self@ == other@ ==> result == 0,
            components_less(other@, self@) ==> result == 1,
    {
        reveal(BackupPath::view);
        let mut component_index = 0;
        while component_index < self.components.len() && component_index < other.components.len()
            invariant
                component_index <= self.components.len(),
                component_index <= other.components.len(),
                self@.take(component_index as int) == other@.take(component_index as int),
            decreases self.components.len() - component_index,
        {
            let left = &self.components[component_index];
            let right = &other.components[component_index];
            let mut byte_index = 0;
            while byte_index < left.len() && byte_index < right.len()
                invariant
                    component_index < self.components.len(),
                    component_index < other.components.len(),
                    self@.take(component_index as int) == other@.take(component_index as int),
                    left@ == self@[component_index as int],
                    right@ == other@[component_index as int],
                    byte_index <= left.len(),
                    byte_index <= right.len(),
                    left@.take(byte_index as int) == right@.take(byte_index as int),
                decreases left.len() - byte_index,
            {
                if left[byte_index] < right[byte_index] {
                    proof {
                        assert(bytes_less(left@, right@)) by {
                            reveal(bytes_less);
                            assert(exists|i: int| 0 <= i < left@.len() && i < right@.len()
                                && left@.take(i) == right@.take(i) && left@[i] < right@[i]) by {
                                assert(left@.take(byte_index as int)
                                    == right@.take(byte_index as int));
                            }
                        }
                        assert(components_less(self@, other@)) by {
                            reveal(components_less);
                            assert(exists|i: int| 0 <= i < self@.len() && i < other@.len()
                                && self@.take(i) == other@.take(i)
                                && bytes_less(self@[i], other@[i])) by {
                                assert(self@.take(component_index as int)
                                    == other@.take(component_index as int));
                                assert(self@[component_index as int] == left@);
                                assert(other@[component_index as int] == right@);
                            }
                        }
                        components_less_is_asymmetric(self@, other@);
                    }
                    return -1;
                }
                if left[byte_index] > right[byte_index] {
                    proof {
                        assert(bytes_less(right@, left@)) by {
                            reveal(bytes_less);
                            assert(exists|i: int| 0 <= i < right@.len() && i < left@.len()
                                && right@.take(i) == left@.take(i) && right@[i] < left@[i]) by {
                                assert(right@.take(byte_index as int)
                                    == left@.take(byte_index as int));
                            }
                        }
                        assert(components_less(other@, self@)) by {
                            reveal(components_less);
                            assert(exists|i: int| 0 <= i < other@.len() && i < self@.len()
                                && other@.take(i) == self@.take(i)
                                && bytes_less(other@[i], self@[i])) by {
                                assert(other@.take(component_index as int)
                                    == self@.take(component_index as int));
                                assert(other@[component_index as int] == right@);
                                assert(self@[component_index as int] == left@);
                            }
                        }
                        components_less_is_asymmetric(other@, self@);
                    }
                    return 1;
                }
                proof {
                    assert(left@.take(byte_index as int + 1) =~= right@.take(byte_index as int + 1));
                }
                byte_index += 1;
            }
            if left.len() < right.len() {
                proof {
                    assert(left@.take(left@.len() as int) =~= left@);
                    assert(bytes_less(left@, right@));
                    assert(components_less(self@, other@)) by {
                        assert(self@.take(component_index as int) == other@.take(component_index as int));
                    }
                    components_less_is_asymmetric(self@, other@);
                }
                return -1;
            }
            if left.len() > right.len() {
                proof {
                    assert(right@.take(right@.len() as int) =~= right@);
                    assert(bytes_less(right@, left@));
                    assert(components_less(other@, self@));
                    components_less_is_asymmetric(other@, self@);
                }
                return 1;
            }
            proof {
                assert(left@.take(byte_index as int) =~= left@);
                assert(right@.take(byte_index as int) =~= right@);
                assert(left@ =~= right@);
                assert(self@.take(component_index as int + 1) =~= other@.take(component_index as int + 1));
            }
            component_index += 1;
        }
        proof {
            if component_index == self.components.len() {
                assert(self@.take(component_index as int) =~= self@);
            }
            if component_index == other.components.len() {
                assert(other@.take(component_index as int) =~= other@);
            }
        }
        if self.components.len() < other.components.len() {
            proof {
                assert(self@.len() < other@.len());
                assert(self@ == other@.take(self@.len() as int));
                assert(components_less(self@, other@));
                components_less_is_asymmetric(self@, other@);
            }
            -1
        } else if self.components.len() > other.components.len() {
            proof {
                assert(other@.len() < self@.len());
                assert(other@ == self@.take(other@.len() as int));
                assert(components_less(other@, self@));
                components_less_is_asymmetric(other@, self@);
            }
            1
        } else {
            0
        }
    }
}
}

//= docs/parsing-requirements.md#parse-state-003
//= type=implementation
//# `SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components. Each component MUST be compared as an unsigned byte sequence using lexicographic byte order, with a proper prefix ordered before the longer sequence. Paths MUST be compared as sequences of components using the same lexicographic rule, with a proper component-sequence prefix ordered before the longer path. Component boundaries MUST remain significant; the ordering MUST NOT be derived by comparing slash-joined or rendered path strings.
impl Ord for BackupPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare(other).cmp(&0)
    }
}

impl PartialOrd for BackupPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for BackupPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.components.is_empty() {
            return f.write_str(".");
        }
        for (index, component) in self.components.iter().enumerate() {
            if index != 0 {
                f.write_str("/")?;
            }
            for byte in component {
                match byte {
                    0x21..=0x7e if *byte != b'\\' => write!(f, "{}", char::from(*byte))?,
                    _ => write!(f, "\\x{byte:02x}")?,
                }
            }
        }
        Ok(())
    }
}
