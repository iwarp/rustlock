#![allow(clippy::redundant_else)]

use ::sysinfo::{Disks, Networks, System};
use ecies::decrypt;
use license::License;
use log::trace;
use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use version_compare::Version;

use crate::error::RustLockErrors;

pub mod error;
pub mod license;
mod sysinfo;

pub struct RustLock {
    license_key: String,
    blocked_customer: Vec<u16>,
    version: String,
    mid_key: String,
    info_key: String,

    network_lock: String,
    storage_lock: String,
    cpu_lock: String,
    os_lock: String,
}

impl RustLock {
    pub fn new(
        license_key: String,
        blocked_customer: Vec<u16>,
        version: String,
        mid_key: String,
        info_key: String,
    ) -> Self {
        let (network_lock, storage_lock, cpu_lock, os_lock) = sysinfo::get_locks(&mid_key);

        Self {
            license_key,
            blocked_customer,
            version,
            mid_key,
            info_key,

            network_lock,
            storage_lock,
            cpu_lock,
            os_lock,
        }
    }

    pub fn get_system_fingerprint(&self) -> String {
        let mut sys = System::new_all();

        // First we update all information of our `System` struct.
        sys.refresh_all();

        let mut lic_info = sysinfo::SysInfo::default();

        // We display all disks' information:
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            // info!("{:?}", disk);
            lic_info
                .storage_name
                .clone_from(&disk.name().to_str().unwrap_or_default().to_owned());
            lic_info.storage_type = format!("{:?}", disk.kind());
        }

        // Network interfaces name, data received and data transmitted:
        let networks = Networks::new_with_refreshed_list();
        for (_interface_name, data) in &networks {
            // info!("{}: {}/{} B", interface_name, data.received(), data.transmitted());
            // info!("Address: {}", data.mac_address());
            lic_info.net = format!("{}", data.mac_address());
        }

        // RAM and swap information:
        // info!("total memory: {} bytes", sys.total_memory());
        lic_info.mem = sys.total_memory();

        // Display system information:
        // info!("System name:             {:?}", sysinfo::System::name());
        // info!("System kernel version:   {:?}", sysinfo::System::kernel_version());
        // info!("System OS version:       {:?}", sysinfo::System::os_version());
        // info!("System host name:        {:?}", sysinfo::System::host_name());

        lic_info.name = System::name().unwrap_or_default();
        lic_info.version = System::os_version().unwrap_or_default();
        lic_info.hostname = System::host_name().unwrap_or_default();

        // Number of CPUs:
        // info!("NB CPUs: {}", sys.cpus().len());

        lic_info.cpu = sys.cpus()[0].brand().to_owned() + " " + sys.cpus()[0].vendor_id();

        // info!("CPU Vendor ID: {}", sys.cpus()[0].vendor_id());
        // info!("Brand: {}", sys.cpus()[0].brand());

        lic_info.c_hash.clone_from(&self.cpu_lock);
        lic_info.o_hash.clone_from(&self.os_lock);
        lic_info.n_hash.clone_from(&self.network_lock);
        lic_info.s_hash.clone_from(&self.storage_lock);

        let os_hash = IdBuilder::new(Encryption::SHA256)
            .add_component(HWIDComponent::OSName)
            .add_component(HWIDComponent::MachineName)
            .build(&self.mid_key)
            .expect("To generate a string");

        if os_hash == lic_info.o_hash {
            lic_info.to_encrypt_string(&self.info_key)
        } else {
            "Failed to generate HWID".to_string()
        }
    }

    /// # Errors
    ///
    /// Will return `Err` if the license isn't valid message as to why its invalid isn't shown
    /// On purpose
    pub fn validate_license(&self, key: &str) -> Result<License, RustLockErrors> {
        let sk = hex::decode(&self.license_key).expect("decode works ok");
        let current_version = Version::from(&self.version).expect("version is created from const");

        let (_network_lock, storage_lock, cpu_lock, os_lock) =
            crate::sysinfo::get_locks(&self.mid_key);

        if let Ok(payload) = hex::decode(key) {
            if let Ok(decrypted) = decrypt(&sk, &payload) {
                // MsgPack
                if let Ok(lic) = rmp_serde::from_read::<&[u8], License>(&*decrypted) {
                    trace!("License: {:?}", lic);

                    #[allow(clippy::if_not_else)]
                    if !self.blocked_customer.contains(&lic.customer) {
                        if let Some(max_version) = Version::from(&lic.version) {
                            if current_version <= max_version {
                                if lic.c1 == os_lock && lic.c2 == cpu_lock && lic.c3 == storage_lock
                                {
                                    return Ok(lic);
                                } else {
                                    trace!("Hardware Locks Failed to match");
                                }
                            } else {
                                trace!("License Version {} <= {}", current_version, max_version);
                            }
                        } else {
                            trace!("License Version Decode Failed");
                        }
                    } else {
                        trace!("License Blocked Customer");
                    }
                } else {
                    trace!("RMP Decode Failed");
                }
            } else {
                trace!("Description Failed");
            }
        } else {
            trace!("License Hex Decode Failed");
        }

        Err(RustLockErrors::InvalidKey)
    }

    /// # Errors
    ///
    /// Will return `Err` if the license isn't valid message as to why its invalid isn't shown
    /// On purpose
    pub fn read_license(&self, key: &str) -> Result<License, RustLockErrors> {
        let sk = hex::decode(&self.info_key).expect("decode works ok");
        if let Ok(payload) = hex::decode(key) {
            if let Ok(decrypted) = decrypt(&sk, &payload) {
                // MsgPack
                if let Ok(lic) = rmp_serde::from_read::<&[u8], License>(&*decrypted) {
                    return Ok(lic);
                } else {
                    trace!("RMP Decode Failed");
                }
            } else {
                trace!("Description Failed");
            }
        } else {
            trace!("License Hex Decode Failed");
        }

        Err(RustLockErrors::InvalidKey)
    }
}
