use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SmtpStatus {
    Accepted,
    AcceptedForwarded,
    CatchAll,
    BadMailbox,
    BadDomain,
    PolicyBlocked,
    MailboxFull,
    MailboxDisabled,
    TempFailure,
    NetworkError,
    ProtocolError,
    Timeout,
    #[default]
    Inconclusive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FinalTriage {
    Alive,
    Dead,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SmtpProbeRecord {
    pub email: String,
    pub outcome: SmtpStatus,
    pub smtp_basic_code: Option<u16>,
    pub smtp_enhanced_code: Option<String>,
    pub smtp_reply_text: Option<String>,
    pub mx_host: Option<String>,
    pub catch_all: bool,
    pub cached: bool,
    pub duration_ms: u64,
}

impl SmtpStatus {
    pub fn is_deliverable(&self) -> bool {
        matches!(self, Self::Accepted | Self::AcceptedForwarded)
    }

    pub fn is_legacy_rejected(&self) -> bool {
        matches!(self, Self::BadMailbox | Self::BadDomain)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "Accepted",
            Self::AcceptedForwarded => "AcceptedForwarded",
            Self::CatchAll => "CatchAll",
            Self::BadMailbox => "BadMailbox",
            Self::BadDomain => "BadDomain",
            Self::PolicyBlocked => "PolicyBlocked",
            Self::MailboxFull => "MailboxFull",
            Self::MailboxDisabled => "MailboxDisabled",
            Self::TempFailure => "TempFailure",
            Self::NetworkError => "NetworkError",
            Self::ProtocolError => "ProtocolError",
            Self::Timeout => "Timeout",
            Self::Inconclusive => "Inconclusive",
        }
    }
}

impl FinalTriage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Alive => "Alive",
            Self::Dead => "Dead",
            Self::Unknown => "Unknown",
        }
    }
}
