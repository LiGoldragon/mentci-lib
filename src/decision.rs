//! The closed-decision -> criome verdict mapping (Spirit t00s).
//!
//! A mentci approval verdict is a CLOSED choice over `signal-mentci`'s
//! [`ApprovalDecision`]; when the question originated as a criome escalation
//! (`ApprovalSource::CriomeEscalation`), answering it means handing criome a
//! `meta-signal-criome` [`AuthorizationApprovalDecision`] keyed by the
//! [`AuthorizationRequestSlot`] criome parked.
//!
//! This mapping is the shared model's job — the daemon's `criome_bridge`
//! currently owns a private copy of it; re-founded here it is the one place
//! both the daemon and any control client read the projection from. It
//! consumes the real criome contract types rather than redefining them
//! (the whole point of the re-founding: no duplicate vocabularies).

use meta_signal_criome::AuthorizationApprovalDecision;
use signal_criome::AuthorizationRequestSlot;
use signal_mentci::ApprovalDecision;

/// A verdict ready to deliver to criome's meta socket: the request slot criome
/// parked, and the closed decision projected into criome's authorization
/// vocabulary. The runtime wraps this in
/// `meta_signal_criome::Input::SubmitAuthorizationApproval`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriomeVerdict {
    request_slot: AuthorizationRequestSlot,
    decision: AuthorizationApprovalDecision,
}

impl CriomeVerdict {
    /// Project a closed mentci [`ApprovalDecision`] onto the criome request
    /// slot it answers. This is the canonical approve/reject/defer ->
    /// criome-verdict mapping (t00s).
    pub fn from_decision(
        request_slot: AuthorizationRequestSlot,
        decision: ApprovalDecision,
    ) -> Self {
        Self {
            request_slot,
            decision: CriomeDecision::from(decision).into_inner(),
        }
    }

    pub fn request_slot(&self) -> &AuthorizationRequestSlot {
        &self.request_slot
    }

    pub fn decision(&self) -> AuthorizationApprovalDecision {
        self.decision
    }
}

/// The named contact point (`skills/enum-contact-points.md`) between
/// `signal-mentci`'s closed verdict set and `meta-signal-criome`'s
/// authorization decision set. Owning the `From` here keeps the two-enum
/// match in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriomeDecision(AuthorizationApprovalDecision);

impl CriomeDecision {
    pub fn into_inner(self) -> AuthorizationApprovalDecision {
        self.0
    }
}

impl From<ApprovalDecision> for CriomeDecision {
    fn from(decision: ApprovalDecision) -> Self {
        Self(match decision {
            ApprovalDecision::ApproveSuggestedAnswer => AuthorizationApprovalDecision::Approve,
            ApprovalDecision::Reject => AuthorizationApprovalDecision::Reject,
            ApprovalDecision::Defer => AuthorizationApprovalDecision::Defer,
        })
    }
}
