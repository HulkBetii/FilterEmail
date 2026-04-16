use serde::Serialize;

use crate::processor::MxStatus;
use crate::smtp_status::SmtpStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputBucket {
    SmtpDeliverable,
    SmtpRejected,
    SmtpCatchAll,
    HasMxSmtpUnknown,
    ARecordFallback,
    Dead,
    Parked,
    Disposable,
    Typo,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DomainVerifyResult {
    pub dns: MxStatus,
    pub smtp: Option<SmtpStatus>,
}

impl DomainVerifyResult {
    pub fn output_bucket(&self) -> OutputBucket {
        match (&self.dns, &self.smtp) {
            (MxStatus::HasMx, Some(SmtpStatus::Deliverable)) => OutputBucket::SmtpDeliverable,
            (MxStatus::HasMx, Some(SmtpStatus::Rejected)) => OutputBucket::SmtpRejected,
            (MxStatus::HasMx, Some(SmtpStatus::CatchAll)) => OutputBucket::SmtpCatchAll,
            (MxStatus::HasMx, Some(SmtpStatus::Inconclusive)) => OutputBucket::HasMxSmtpUnknown,
            (MxStatus::HasMx, None) => OutputBucket::HasMxSmtpUnknown,
            (MxStatus::ARecordFallback, _) => OutputBucket::ARecordFallback,
            (MxStatus::Dead, _) => OutputBucket::Dead,
            (MxStatus::Parked, _) => OutputBucket::Parked,
            (MxStatus::Disposable, _) => OutputBucket::Disposable,
            (MxStatus::TypoSuggestion(_), _) => OutputBucket::Typo,
            (MxStatus::Inconclusive, _) => OutputBucket::Inconclusive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_bucket_maps_dns_and_smtp_layers() {
        let deliverable = DomainVerifyResult {
            dns: MxStatus::HasMx,
            smtp: Some(SmtpStatus::Deliverable),
        };
        let rejected = DomainVerifyResult {
            dns: MxStatus::HasMx,
            smtp: Some(SmtpStatus::Rejected),
        };
        let unknown = DomainVerifyResult {
            dns: MxStatus::HasMx,
            smtp: None,
        };
        let dead = DomainVerifyResult {
            dns: MxStatus::Dead,
            smtp: Some(SmtpStatus::Rejected),
        };

        assert_eq!(deliverable.output_bucket(), OutputBucket::SmtpDeliverable);
        assert_eq!(rejected.output_bucket(), OutputBucket::SmtpRejected);
        assert_eq!(unknown.output_bucket(), OutputBucket::HasMxSmtpUnknown);
        assert_eq!(dead.output_bucket(), OutputBucket::Dead);
    }
}
