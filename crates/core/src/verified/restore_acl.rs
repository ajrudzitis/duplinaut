//! Evidence gate for inherited ACL absence before restore mode changes.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AclAttribute {
    Access,
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AclClearEvidence {
    pub directory: bool,
    pub access_absent: bool,
    pub default_absent: bool,
}

pub open spec fn acl_clear_evidence_model(
    evidence: AclClearEvidence,
    attribute: AclAttribute,
) -> Option<AclClearEvidence> {
    match attribute {
        AclAttribute::Access => {
            if evidence.access_absent || evidence.default_absent {
                None
            } else {
                Some(AclClearEvidence {
                    directory: evidence.directory,
                    access_absent: true,
                    default_absent: false,
                })
            }
        },
        AclAttribute::Default => {
            if evidence.directory && evidence.access_absent && !evidence.default_absent {
                Some(AclClearEvidence {
                    directory: true,
                    access_absent: true,
                    default_absent: true,
                })
            } else {
                None
            }
        },
    }
}

pub open spec fn acl_clear_complete_model(evidence: AclClearEvidence) -> bool {
    evidence.access_absent
        && (!evidence.directory || evidence.default_absent)
        && (evidence.directory || !evidence.default_absent)
}

pub(crate) fn initial_acl_clear_evidence(directory: bool) -> (evidence: AclClearEvidence)
    ensures evidence == (AclClearEvidence {
        directory,
        access_absent: false,
        default_absent: false,
    }),
{
    AclClearEvidence { directory, access_absent: false, default_absent: false }
}

//= docs/parsing-requirements.md#restore-016
//= type=implementation
//# The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.
pub(crate) fn record_acl_absence(
    evidence: AclClearEvidence,
    attribute: AclAttribute,
) -> (next: Option<AclClearEvidence>)
    ensures next == acl_clear_evidence_model(evidence, attribute),
{
    match attribute {
        AclAttribute::Access => {
            if evidence.access_absent || evidence.default_absent {
                None
            } else {
                Some(AclClearEvidence {
                    directory: evidence.directory,
                    access_absent: true,
                    default_absent: false,
                })
            }
        },
        AclAttribute::Default => {
            if evidence.directory && evidence.access_absent && !evidence.default_absent {
                Some(AclClearEvidence {
                    directory: true,
                    access_absent: true,
                    default_absent: true,
                })
            } else {
                None
            }
        },
    }
}

pub(crate) fn acl_clear_complete(evidence: AclClearEvidence) -> (complete: bool)
    ensures complete == acl_clear_complete_model(evidence),
{
    evidence.access_absent
        && (!evidence.directory || evidence.default_absent)
        && (evidence.directory || !evidence.default_absent)
}

}

#[cfg(test)]
mod tests {
    use super::{acl_clear_complete, initial_acl_clear_evidence, record_acl_absence, AclAttribute};

    #[test]
    //= docs/parsing-requirements.md#restore-016
    //= type=test
    //# The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.
    fn acl_clear_requires_absence_evidence_for_each_applicable_attribute() {
        let file = initial_acl_clear_evidence(false);
        assert!(!acl_clear_complete(file));
        let file = record_acl_absence(file, AclAttribute::Access).unwrap();
        assert!(acl_clear_complete(file));
        assert!(record_acl_absence(file, AclAttribute::Access).is_none());
        assert!(record_acl_absence(file, AclAttribute::Default).is_none());

        let directory = initial_acl_clear_evidence(true);
        assert!(record_acl_absence(directory, AclAttribute::Default).is_none());
        let directory = record_acl_absence(directory, AclAttribute::Access).unwrap();
        assert!(!acl_clear_complete(directory));
        let directory = record_acl_absence(directory, AclAttribute::Default).unwrap();
        assert!(acl_clear_complete(directory));
        assert!(record_acl_absence(directory, AclAttribute::Default).is_none());
    }
}
