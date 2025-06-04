use ecies::encrypt;
use machineid_rs::HWIDComponent;
use machineid_rs::{Encryption, IdBuilder};
use serde::{Deserialize, Serialize};

use crate::error::RustLockErrors;

/// # Errors
/// Will return `Err` if the we cant generate a fingerprint for this pc
pub fn get_locks(mid_key: &str) -> Result<(String, String, String, String), RustLockErrors> {
    let net_lock = IdBuilder::new(Encryption::SHA256).add_component(HWIDComponent::MacAddress).build(mid_key).map_err(|_| RustLockErrors::HWInfoFailed)?;

    let storage_lock = IdBuilder::new(Encryption::SHA256).add_component(HWIDComponent::DriveSerial).build(mid_key).map_err(|_| RustLockErrors::HWInfoFailed)?;

    let cpu_lock = IdBuilder::new(Encryption::SHA256).add_component(HWIDComponent::CPUID).add_component(HWIDComponent::CPUCores).build(mid_key).map_err(|_| RustLockErrors::HWInfoFailed)?;

    let os_lock = IdBuilder::new(Encryption::SHA256)
        .add_component(HWIDComponent::OSName)
        .add_component(HWIDComponent::MachineName)
        .build(mid_key)
        .map_err(|_| RustLockErrors::HWInfoFailed)?;

    Ok((net_lock, storage_lock, cpu_lock, os_lock))
}

#[derive(Serialize, Deserialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct SysInfo {
    pub storage_name: String,
    pub storage_type: String,

    pub mem: u64,

    pub name: String,
    pub version: String,
    pub hostname: String,

    pub cpu: String,

    pub net: String,

    pub c_hash: String,
    pub o_hash: String,
    pub n_hash: String,
    pub s_hash: String,
}

impl SysInfo {
    #[must_use]
    pub(crate) fn to_encrypt_string(&self, info_key: &str) -> String {
        if let Ok(msg) = rmp_serde::to_vec(&self) {
            if let Ok(pk) = hex::decode(info_key) {
                if let Ok(encrypted) = encrypt(&pk, &msg) {
                    let encrypted_string = hex::encode_upper(encrypted);

                    return encrypted_string;
                }
            }
        }

        String::new()
    }
}
