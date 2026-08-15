//! What a run concluded.

/// Whether langbank is level with the upstream.
///
/// `check` exits non-zero on `Incomplete`, which is what makes drift a red
/// build rather than a line of output nobody reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Incomplete,
}

impl Outcome {
    pub fn of(missing: usize) -> Self {
        if missing == 0 {
            Self::Complete
        } else {
            Self::Incomplete
        }
    }
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
