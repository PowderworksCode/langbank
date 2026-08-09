//! What language a file is, and what said so.
//!
//! Detection is deliberately evidence-carrying rather than a bare answer. A
//! consumer that disagrees with a decision can see which rule produced it, and
//! a file identified by shebang is a different kind of claim from one
//! identified by extension.

use serde::{Deserialize, Serialize};

use crate::LanguageId;

/// Why a file was identified as a language. Each variant is a distinct rule,
/// and a detection may carry more than one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LanguageEvidence {
    Extension { extension: String },
    Filename { filename: String },
    Shebang { interpreter: String },
}

/// A decision, plus the evidence for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageDetection {
    pub language: LanguageId,
    pub evidence: Vec<LanguageEvidence>,
}
