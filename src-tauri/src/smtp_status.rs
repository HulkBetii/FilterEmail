use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SmtpStatus {
    Deliverable,
    Rejected,
    CatchAll,
    Inconclusive,
}
