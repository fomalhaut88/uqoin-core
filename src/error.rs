/// Uqoin kinds of error.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    CoinInvalid,
    CoinNotUnique,
    CoinTooCheap,
    TransactionInvalidSender,
    TransactionSelfTransfer,
    TransactionEmpty,
    TransactionBrokenGroup,
    TransactionBrokenExt,
    BlockBroken,
    BlockOrderMismatch,
    BlockValidatorMismatch,
    BlockPreviousHashMismatch,
    BlockOffsetMismatch,
    BlockInvalidHash,
    BlockInvalidHashComplexity,
    Other,
}


/// Shortcut for converting boolean check into error.
#[macro_export]
macro_rules! validate {
    ($check:expr, $kind:ident) => (
        if $check {
            Ok::<(), crate::error::Error>(())
        } else {
            Err(crate::error::ErrorKind::$kind.into())
        }
    )
}


/// Uqoin error structure. It supports converting into `std::io::Error`.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}


impl Error {
    /// Create a new Uqoin error instance.
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    /// Get kind of the error.
    pub fn kind(&self) -> ErrorKind {
        self.kind.clone()
    }
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}


impl From<ErrorKind> for Error {
    fn from(uqoin_error_kind: ErrorKind) -> Error {
        let message = format!("{:?}", uqoin_error_kind);
        Error::new(uqoin_error_kind, message)
    }
}


impl From<Error> for std::io::Error {
    fn from(uqoin_error: Error) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, uqoin_error.to_string())
    }
}


impl From<ErrorKind> for std::io::Error {
    fn from(uqoin_error_kind: ErrorKind) -> std::io::Error {
        let uqoin_error = Error::from(uqoin_error_kind);
        uqoin_error.into()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let err = Error::new(
            ErrorKind::CoinInvalid,
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
                .to_string()
        );

        assert_eq!(err.kind(), ErrorKind::CoinInvalid);
        assert_eq!(
            err.to_string(),
            "E764663DA70C4805F07F733C2A782116C7492C70EE67DD39C5DDA817816B8AB2"
        );
    }

    #[test]
    fn test_err_to_std() {
        let err = Error::new(
            ErrorKind::CoinInvalid,
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
        let kind = ErrorKind::CoinInvalid;
        let err: Error = kind.into();
        assert_eq!(err.kind(), ErrorKind::CoinInvalid);
        assert_eq!(err.to_string(), "CoinInvalid");
    }

    #[test]
    fn test_kind_to_err_str() {
        let kind = ErrorKind::CoinInvalid;
        let err_std: std::io::Error = kind.into();
        assert_eq!(err_std.kind(), std::io::ErrorKind::Other);
        assert_eq!(err_std.to_string(), "CoinInvalid");
    }
}
