//! Parse error types for SysML v2 parser.

/// Error returned when parsing fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable description of the error.
    pub message: String,
    /// Optional byte offset in the input where the error occurred.
    pub offset: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset: None,
        }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.offset {
            Some(off) => write!(f, "{} at offset {}", self.message, off),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}
