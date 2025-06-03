use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Default, Debug, Eq, PartialEq, Clone)]
pub struct License {
    pub start_month: u32,
    pub start_year: i32,

    pub end_month: u32,
    pub end_year: i32,

    pub version: String,

    pub customer: u16,

    // feature flags
    pub f1: bool, // 1
    pub f2: bool, // 4
    pub f3: bool, // 8
    pub f4: bool, // unlimited
    pub f5: bool,

    // controls
    pub c1: String,
    pub c2: String,
    pub c3: String,
    pub c4: String,
    pub c5: String,

    pub id: String,
    pub name: String,
}
