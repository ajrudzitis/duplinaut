//! Verified guard for ordinary versus multipart representation reuse.
use vstd::prelude::*;

verus! {

pub open spec fn representation_admission_model(
    previous: Option<bool>,
    incoming_multipart: bool,
) -> bool {
    match previous {
        None => true,
        Some(false) => !incoming_multipart,
        Some(true) => false,
    }
}

//= docs/parsing-requirements.md#parse-multi-006
//= type=implementation
//# Assembly MUST reject an ordinary/multipart representation mixture or a second multipart logical entry for one path within the initial supported backup-set profile.
pub fn admit_representation(
    previous: Option<bool>,
    incoming_multipart: bool,
) -> (admitted: bool)
    ensures admitted == representation_admission_model(previous, incoming_multipart),
{
    match previous {
        None => true,
        Some(false) => !incoming_multipart,
        Some(true) => false,
    }
}

}

#[cfg(test)]
mod tests {
    use super::admit_representation;

    #[test]
    //= docs/parsing-requirements.md#parse-multi-006
    //= type=test
    //# Assembly MUST reject an ordinary/multipart representation mixture or a second multipart logical entry for one path within the initial supported backup-set profile.
    fn representation_admission_covers_all_marker_and_kind_combinations() {
        let cases = [
            (None, false, true),
            (None, true, true),
            (Some(false), false, true),
            (Some(false), true, false),
            (Some(true), false, false),
            (Some(true), true, false),
        ];
        assert_eq!(cases.len(), 6);
        for (previous, incoming_multipart, expected) in cases {
            assert_eq!(
                admit_representation(previous, incoming_multipart),
                expected,
                "marker={previous:?}, incoming_multipart={incoming_multipart}"
            );
        }
    }
}
