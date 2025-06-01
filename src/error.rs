use thiserror::Error;

#[derive(Error, Debug)]
pub enum RustLockErrors {
    #[error("License Key Error")]
    InvalidKey,
}
