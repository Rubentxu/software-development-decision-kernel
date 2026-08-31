//! Structured error envelope (RNF-006): stable code, context, cause, recovery.

/// Errors that can report a stable code and a suggested recovery action.
pub trait SddkErrorCode: std::error::Error {
    /// Stable machine-readable error code.
    fn code(&self) -> &'static str;
    /// Suggested recovery action.
    fn recovery(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::SddkErrorCode;

    #[derive(Debug, Error)]
    enum SampleError {
        #[error("thing broke: {detail}")]
        Named { detail: String },
        #[error("other broke")]
        Tuple(String),
        #[error("unit broke")]
        Unit,
    }

    impl SddkErrorCode for SampleError {
        fn code(&self) -> &'static str {
            match self {
                Self::Named { .. } => "SAMPLE_NAMED",
                Self::Tuple(..) => "SAMPLE_TUPLE",
                Self::Unit => "SAMPLE_UNIT",
            }
        }

        fn recovery(&self) -> &'static str {
            match self {
                Self::Named { .. } => "inspect the detail and retry",
                Self::Tuple(..) => "check the tuple value",
                Self::Unit => "retry the operation",
            }
        }
    }

    #[test]
    fn codes_and_recoveries_are_stable() {
        let named = SampleError::Named { detail: "x".into() };
        assert_eq!(named.code(), "SAMPLE_NAMED");
        assert_eq!(named.recovery(), "inspect the detail and retry");
        let tuple = SampleError::Tuple("y".into());
        assert_eq!(tuple.code(), "SAMPLE_TUPLE");
        let unit = SampleError::Unit;
        assert_eq!(unit.code(), "SAMPLE_UNIT");
        assert_eq!(unit.recovery(), "retry the operation");
    }
}
