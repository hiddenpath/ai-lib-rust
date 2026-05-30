use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssembleError {
    EmptyInput,
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "no messages to assemble"),
        }
    }
}

impl std::error::Error for AssembleError {}
