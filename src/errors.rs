/// Uqoin kinds of error.
#[derive(Debug, Clone, PartialEq)]
pub enum UqoinErrorKind {
    CoinInvalid,
    CoinNotUnique,
    CoinTooCheap,
    TransactionInvalidSender,
    TransactionSelfTransfer,
    BlockBroken,
    BlockOrderMismatch,
    BlockValidatorMismatch,
    BlockPreviousHashMismatch,
    Other,
}


/// Uqoin error structure. It supports converting into `std::io::Error`.
#[derive(Debug, Clone, PartialEq)]
pub struct UqoinError {
    kind: UqoinErrorKind,
    message: String,
}


impl UqoinError {
    /// Create a new Uqoin error instance.
    pub fn new(kind: UqoinErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    /// Get kind of the error.
    pub fn kind(&self) -> UqoinErrorKind {
        self.kind.clone()
    }
}


impl std::fmt::Display for UqoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}


impl From<UqoinErrorKind> for UqoinError {
    fn from(uqoin_error_kind: UqoinErrorKind) -> UqoinError {
        let message = format!("{:?}", uqoin_error_kind);
        UqoinError::new(uqoin_error_kind, message)
    }
}


impl From<UqoinError> for std::io::Error {
    fn from(uqoin_error: UqoinError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, uqoin_error.to_string())
    }
}


impl From<UqoinErrorKind> for std::io::Error {
    fn from(uqoin_error_kind: UqoinErrorKind) -> std::io::Error {
        let uqoin_error = UqoinError::from(uqoin_error_kind);
        uqoin_error.into()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let err = UqoinError::new(
            UqoinErrorKind::CoinInvalid,
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
                .to_string()
        );

        assert_eq!(err.kind(), UqoinErrorKind::CoinInvalid);
        assert_eq!(
            err.to_string(),
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
        );
    }

    #[test]
    fn test_err_to_std() {
        let err = UqoinError::new(
            UqoinErrorKind::CoinInvalid,
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
                .to_string()
        );

        let err_std: std::io::Error = err.into();

        assert_eq!(err_std.kind(), std::io::ErrorKind::Other);
        assert_eq!(
            err_std.to_string(), 
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
        );
    }

    #[test]
    fn test_kind_to_err() {
        let kind = UqoinErrorKind::CoinInvalid;
        let err: UqoinError = kind.into();
        assert_eq!(err.kind(), UqoinErrorKind::CoinInvalid);
        assert_eq!(err.to_string(), "CoinInvalid");
    }

    #[test]
    fn test_kind_to_err_str() {
        let kind = UqoinErrorKind::CoinInvalid;
        let err_std: std::io::Error = kind.into();
        assert_eq!(err_std.kind(), std::io::ErrorKind::Other);
        assert_eq!(err_std.to_string(), "CoinInvalid");
    }
}
