use crate::processor::MxStatus;
use crate::smtp_status::{FinalTriage, SmtpProbeRecord, SmtpStatus};

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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainVerifyResult {
    pub dns: MxStatus,
    pub smtp: Option<SmtpProbeRecord>,
}

#[allow(dead_code)]
impl DomainVerifyResult {
    pub fn output_bucket(&self) -> OutputBucket {
        output_bucket_for(&self.dns, self.smtp.as_ref())
    }
}

pub fn output_bucket_for(dns: &MxStatus, smtp: Option<&SmtpProbeRecord>) -> OutputBucket {
    match (dns, smtp.map(|record| &record.outcome)) {
        (MxStatus::HasMx, Some(status)) if status.is_deliverable() => OutputBucket::SmtpDeliverable,
        (MxStatus::HasMx, Some(SmtpStatus::CatchAll)) => OutputBucket::SmtpCatchAll,
        (MxStatus::HasMx, Some(status)) if status.is_legacy_rejected() => OutputBucket::SmtpRejected,
        (MxStatus::HasMx, _) => OutputBucket::HasMxSmtpUnknown,
        (MxStatus::ARecordFallback, _) => OutputBucket::ARecordFallback,
        (MxStatus::Dead | MxStatus::NullMx, _) => OutputBucket::Dead,
        (MxStatus::Parked, _) => OutputBucket::Parked,
        (MxStatus::Disposable, _) => OutputBucket::Disposable,
        (MxStatus::TypoSuggestion(_), _) => OutputBucket::Typo,
        (MxStatus::Inconclusive, _) => OutputBucket::Inconclusive,
    }
}

pub fn final_triage_for(dns: &MxStatus, smtp: Option<&SmtpProbeRecord>) -> FinalTriage {
    match dns {
        MxStatus::Dead | MxStatus::NullMx => FinalTriage::Dead,
        MxStatus::ARecordFallback
        | MxStatus::Parked
        | MxStatus::Disposable
        | MxStatus::TypoSuggestion(_)
        | MxStatus::Inconclusive => FinalTriage::Unknown,
        MxStatus::HasMx => match smtp {
            Some(record) if record.catch_all => FinalTriage::Unknown,
            Some(record) => match record.outcome {
                SmtpStatus::Accepted | SmtpStatus::AcceptedForwarded => FinalTriage::Alive,
                SmtpStatus::BadMailbox | SmtpStatus::BadDomain => FinalTriage::Dead,
                SmtpStatus::CatchAll
                | SmtpStatus::PolicyBlocked
                | SmtpStatus::MailboxFull
                | SmtpStatus::MailboxDisabled
                | SmtpStatus::TempFailure
                | SmtpStatus::NetworkError
                | SmtpStatus::ProtocolError
                | SmtpStatus::Timeout
                | SmtpStatus::Inconclusive => FinalTriage::Unknown,
            },
            None => FinalTriage::Unknown,
        },
    }
}

pub fn dns_status_name(status: &MxStatus) -> String {
    match status {
        MxStatus::HasMx => "HasMx".to_string(),
        MxStatus::ARecordFallback => "ARecordFallback".to_string(),
        MxStatus::Dead => "Dead".to_string(),
        MxStatus::NullMx => "NullMx".to_string(),
        MxStatus::Parked => "Parked".to_string(),
        MxStatus::Disposable => "Disposable".to_string(),
        MxStatus::TypoSuggestion(suggestion) => format!("TypoSuggestion({suggestion})"),
        MxStatus::Inconclusive => "Inconclusive".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_bucket_maps_dns_and_smtp_layers() {
        let deliverable = SmtpProbeRecord {
            email: "person@gmail.com".to_string(),
            outcome: SmtpStatus::Accepted,
            ..Default::default()
        };
        let rejected = SmtpProbeRecord {
            email: "person@gmail.com".to_string(),
            outcome: SmtpStatus::BadMailbox,
            ..Default::default()
        };
        let unknown = SmtpProbeRecord {
            email: "person@gmail.com".to_string(),
            outcome: SmtpStatus::TempFailure,
            ..Default::default()
        };

        assert_eq!(
            output_bucket_for(&MxStatus::HasMx, Some(&deliverable)),
            OutputBucket::SmtpDeliverable
        );
        assert_eq!(
            output_bucket_for(&MxStatus::HasMx, Some(&rejected)),
            OutputBucket::SmtpRejected
        );
        assert_eq!(
            output_bucket_for(&MxStatus::HasMx, Some(&unknown)),
            OutputBucket::HasMxSmtpUnknown
        );
        assert_eq!(
            output_bucket_for(&MxStatus::NullMx, None),
            OutputBucket::Dead
        );
    }

    #[test]
    fn final_triage_maps_conservatively() {
        let alive = SmtpProbeRecord {
            email: "alive@gmail.com".to_string(),
            outcome: SmtpStatus::Accepted,
            ..Default::default()
        };
        let dead = SmtpProbeRecord {
            email: "dead@gmail.com".to_string(),
            outcome: SmtpStatus::BadMailbox,
            ..Default::default()
        };
        let unknown = SmtpProbeRecord {
            email: "unknown@gmail.com".to_string(),
            outcome: SmtpStatus::PolicyBlocked,
            ..Default::default()
        };

        assert_eq!(final_triage_for(&MxStatus::HasMx, Some(&alive)), FinalTriage::Alive);
        assert_eq!(final_triage_for(&MxStatus::HasMx, Some(&dead)), FinalTriage::Dead);
        assert_eq!(
            final_triage_for(&MxStatus::HasMx, Some(&unknown)),
            FinalTriage::Unknown
        );
        assert_eq!(final_triage_for(&MxStatus::ARecordFallback, None), FinalTriage::Unknown);
        assert_eq!(final_triage_for(&MxStatus::NullMx, None), FinalTriage::Dead);
    }

    #[test]
    fn final_triage_covers_dead_and_unknown_smtp_outcomes() {
        let bad_domain = SmtpProbeRecord {
            email: "dead@gmail.com".to_string(),
            outcome: SmtpStatus::BadDomain,
            ..Default::default()
        };
        let policy = SmtpProbeRecord {
            email: "policy@gmail.com".to_string(),
            outcome: SmtpStatus::PolicyBlocked,
            ..Default::default()
        };
        let temp_fail = SmtpProbeRecord {
            email: "temp@gmail.com".to_string(),
            outcome: SmtpStatus::TempFailure,
            ..Default::default()
        };
        let catch_all = SmtpProbeRecord {
            email: "catch@gmail.com".to_string(),
            outcome: SmtpStatus::CatchAll,
            catch_all: true,
            ..Default::default()
        };

        assert_eq!(
            final_triage_for(&MxStatus::HasMx, Some(&bad_domain)),
            FinalTriage::Dead
        );
        assert_eq!(
            final_triage_for(&MxStatus::HasMx, Some(&policy)),
            FinalTriage::Unknown
        );
        assert_eq!(
            final_triage_for(&MxStatus::HasMx, Some(&temp_fail)),
            FinalTriage::Unknown
        );
        assert_eq!(
            final_triage_for(&MxStatus::HasMx, Some(&catch_all)),
            FinalTriage::Unknown
        );
    }
}
