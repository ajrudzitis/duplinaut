//! Verified final-publication decision for provisional multipart assembly.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishGate {
    Poisoned,
    Incomplete,
    Ready,
}

pub open spec fn finish_gate_model(
    failed: bool,
    admission_complete: bool,
    pending: bool,
) -> FinishGate {
    if failed {
        FinishGate::Poisoned
    } else if !admission_complete || pending {
        FinishGate::Incomplete
    } else {
        FinishGate::Ready
    }
}

//= docs/parsing-requirements.md#parse-multi-003
//= type=implementation
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
pub fn finish_gate(
    failed: bool,
    admission_complete: bool,
    pending: bool,
) -> (result: FinishGate)
    ensures result == finish_gate_model(failed, admission_complete, pending),
{
    if failed {
        FinishGate::Poisoned
    } else if !admission_complete || pending {
        FinishGate::Incomplete
    } else {
        FinishGate::Ready
    }
}

}

#[cfg(test)]
mod tests {
    use super::{finish_gate, FinishGate};

    #[test]
    // This table checks only the final gate over its three supplied observations;
    // it does not establish how admission or pending state was produced.
    //= docs/parsing-requirements.md#parse-multi-003
    //= type=test
    //# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
    fn finish_gate_exhaustively_preserves_poison_precedence() {
        let cases = [
            (false, false, false, FinishGate::Incomplete),
            (false, false, true, FinishGate::Incomplete),
            (false, true, false, FinishGate::Ready),
            (false, true, true, FinishGate::Incomplete),
            (true, false, false, FinishGate::Poisoned),
            (true, false, true, FinishGate::Poisoned),
            (true, true, false, FinishGate::Poisoned),
            (true, true, true, FinishGate::Poisoned),
        ];
        assert_eq!(cases.len(), 8);
        for (failed, admission_complete, pending, expected) in cases {
            assert_eq!(
                finish_gate(failed, admission_complete, pending),
                expected,
                "finish gate for failed={failed}, admission_complete={admission_complete}, pending={pending}"
            );
        }
    }
}
