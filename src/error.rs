//! The `error` module in the `uqoin-cor`e library defines a structured approach
//! to error handling within the Uqoin cryptocurrency protocol. It introduces a 
//! comprehensive enumeration of error kinds that represent various failure 
//! scenarios encountered during protocol operations, such as coin validation, 
//! transaction processing, and block verification. By encapsulating these error
//! conditions, the module facilitates robust error management and propagation
//! throughout the system.

/// Represents specific categories of errors that can occur within the Uqoin 
/// protocol:
/// * CoinInvalid: Indicates that a coin fails validation checks.
/// * CoinNotUnique: Denotes duplication of coin identifiers.
/// * CoinTooCheap: Signifies that a coin's value is below the acceptable 
/// threshold.
/// * TransactionInvalidSender: The sender information in a transaction is 
/// invalid or cannot be verified.
/// * TransactionEmpty: The transaction contains no operations or data.
/// * TransactionBrokenGroup: The transaction group structure is malformed or 
/// inconsistent.
/// * TransactionBrokenExt: Extension data is corrupted or invalid.
/// * BlockBroken: The block structure is corrupted or fails integrity checks.
/// * BlockOrderMismatch: The sequence of blocks does not follow the expected 
/// order.
/// * BlockValidatorMismatch: The block's validator does not match the expected
/// validator.
/// * BlockPreviousHashMismatch: The previous hash reference in the block does
/// not match the actual previous block's hash.
/// * BlockOffsetMismatch: The block's offset value is incorrect or
/// inconsistent.
/// * BlockInvalidHash: The block's hash does not meet the required criteria.
/// * BlockInvalidHashComplexity: The block's hash does not satisfy the 
/// complexity requirements.
/// * Other: A catch-all for unspecified or miscellaneous errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    CoinInvalid,
    CoinNotUnique,
    CoinTooCheap,
    TransactionInvalidSender,
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
/// A utility macro to streamline error checking:
/// ```ignore
/// validate!(condition, ErrorKindVariant)
/// ```
/// If condition evaluates to `true`, it returns `Ok(())`; otherwise, it returns
/// an `Err` with the specified `ErrorKind`.
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
/// Encapsulates an error kind along with a descriptive message:
/// * kind: An instance of ErrorKind representing the type of error.
/// * message: A human-readable description of the error.
/// Implements the `std::error::Error` and `std::fmt::Display` traits for 
/// integration with Rust's error handling ecosystem.
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


impl std::error::Error for Error {}


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

    #[test]
    fn test_validate_macro() {
        let result = validate!(true, CoinInvalid);
        assert!(result.is_ok());

        let result = validate!(false, CoinTooCheap);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::CoinTooCheap);
        }
    }
}
