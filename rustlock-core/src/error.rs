use thiserror::Error;

#[derive(Error, Debug)]
pub enum RustLockErrors {
    #[error("License Key Error")]
    InvalidKey,
    #[error("Invalid Input Version")]
    InvalidVersion,
    #[error("License Input Public Key")]
    InvalidPublicKey,
    #[error("License Input Hex Decode")]
    InvalidHexDecode,
    #[error("License Decrypt")]
    InvalidDecrypt,
    #[error("Failed to Generate HW Info")]
    HWInfoFailed,
}
