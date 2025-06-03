#![allow(clippy::unwrap_used, clippy::field_reassign_with_default, clippy::option_if_let_else)]

use chrono::prelude::*;
use env_logger::{Builder, Env};
use log::info;
use version_compare::Version;

use ecies::{encrypt, utils::generate_keypair};

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(version, about="License Admin Tool for RustLock", long_about = None)]
#[command(propagate_version = true)]
struct Opts {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new key
    Generate {
        #[clap(long)]
        hwid: String,

        /// Number of years to have upgrade support
        #[clap(long, default_value_t = 1)]
        support: i32,

        /// Customer ID
        #[clap(long)]
        customer: u32,

        /// Customer Name
        #[clap(long)]
        name: String,
    },
    /// Validate a key
    Validate { code: String },
    /// Generate a new keypair.
    Keys,
}

const INFO_SECRET_KEY_STRING: &str = "28EA8E7C9AC0949C17AFC2D6C847DE3C008905FC546140CCEC6450428CFAB743";

#[allow(dead_code)]
const INFO_PRIVATE_KEY_STRING: &str = "04EB54A3A795375E5F5AB75071911CD444F3724396435D3634D076A22FC30E380C08F78B6839BB1877A28999FAECE9BDAD01F4965B852A8C8DD615B49D9A57F487";

#[allow(dead_code)]
const LIC_SECRET_KEY_STRING: &str = "0312F2E50CA3B5F731F542802E9DA71DEE287FDB0AE06F4C612291707E43B970";
const LIC_PRIVATE_KEY_STRING: &str = "0434A2FB992523C8C61D2747DC7E9DA84D72857AB56F63569B2AB6DBAD2FE6A96C89B370A1A70F8185CE4EAB166C4261E87FCF21B93E54FF9A7DEABCC3AB0B458A";

fn main() {
    Builder::from_env(Env::default().default_filter_or("trace")).init();

    let opts = Opts::parse();

    match opts.command {
        Commands::Generate { hwid, support, customer, name } => issue(&hwid, support, customer, name),
        Commands::Validate { code } => validate(&code),
        Commands::Keys => generate_new_secrets(),
    }
}

fn validate(code: &str) {
    info!("Validating Code: {code}");

    if let Ok(lic) = libptznet::license::License::validate_license(code) {
        info!("License Decoded: {lic:#?}");
    } else {
        info!("Failed to Decode License");
    }
}

fn issue(hwid: &str, support: i32, customer: u32, name: String) {
    info!("Support Years {}", support);
    info!("CustomerID: {:?}", customer);
    info!("Customer Name: {:?}", name);
    info!("HWID: {:?}", hwid);

    let Some(sysinfo) = SysInfo::decode_from_string(hwid, INFO_SECRET_KEY_STRING) else {
        info!("*** Failed to Load HWInfo ***");
        info!("\n Quitting...");
        return;
    };

    header("Decrypting HWInfo Object");
    info!("HWInfo: {sysinfo:#?}");

    header("Updating License Object");

    let mut lic = libptznet::license::License::default();

    lic.name = name;

    let current_version = Version::from(libptznet::license::VERSION).unwrap();
    // set max version
    lic.version = current_version.part(0).unwrap().to_string() + "." + &current_version.part(1).unwrap().to_string() + ".9999";

    let date = Utc::now();

    lic.customer = 1;
    lic.start_month = date.month();
    lic.start_year = date.year();

    lic.end_month = date.month();
    lic.end_year = date.year() + support;

    lic.c1 = sysinfo.o_hash;
    lic.c2 = sysinfo.c_hash;
    lic.c3 = sysinfo.s_hash;
    lic.c4 = sysinfo.n_hash;

    // update features
    lic.f1 = false; // 1
    lic.f2 = true; // 4
    lic.f3 = false; // 8
    lic.f4 = false; // 9999
    lic.f5 = false; // unused

    info!("{lic:?}");

    header("Encrypting License Object");

    let lic_pk = hex::decode(LIC_PRIVATE_KEY_STRING).unwrap();
    let msg = rmp_serde::to_vec(&lic).unwrap();

    let encrypted = encrypt(&lic_pk, &msg).unwrap();
    let encrypted_string = hex::encode_upper(encrypted);
    info!("Done");

    header("Testing Serial Decode");

    if let Ok(lic) = libptznet::license::License::validate_license(&encrypted_string) {
        info!("License Decoded: {lic:#?}");
    } else {
        info!("Failed to Decode License");
    }

    header("Working Serial Number");
    info!("{}", encrypted_string.bright_yellow());

    // let v1 = Version::from(event_hub_common::license::VERSION).unwrap();
    // // let v1 = Version::from("0.3").unwrap();
    // let v2 = Version::from(&lic.version).unwrap();
    // info!("v1: {v1} v2: {v2}");

    // if v1 <= v2 {
    //     info!("current version is less than our max version");
    // } else {
    //     info!("current version is higher than our max version");
    // }

    // assert_eq!(
    //     true,
    //     event_hub_common::licence::License::validate_license(
    //         &"044FC1C0E0440A7C131E06404DE2C8F01C961447BBC32DB8F3EB439CB79CDBB0CB95B0BA57F6388D41CECE7101EA0F8C607819412DA4B19A7C92133EFC0290CE7DC0079AEA3A60ACEABD9AF496FEDFABAA3696B8C8DD978385CCA4D1C856B6641DB07740C08D3DD5AC1D77843FB25D6E6323A1A4DB3FAA1B8E70A04A759C030037D94C5C97E59BB8EF389287F7"
    //             .to_string()
    //     )
    //     .is_err()
    // );
    // info!("Test OK: Working License");

    // assert_eq!(
    //     true,
    //     event_hub_common::licence::License::validate_license(
    //         &"04B57036F2020EAAD8032E9F8088DF7E775E8557A8B85B36C158DD2F781653E257D74A8E8388AC6CCA754D5F93C90A024E96446808E5300C637981658BAFBA89EC62680E10368CDA415F3AAB0FA4F72CC5C27659CF45845DB08938D1AC2F84D12C087EE545F723674EF3C48160F636E9D4AAEDEF9884E3EEC640A120F4318633A6D6FEEBC3BB529C"
    //             .to_string()
    //     )
    //     .is_err()
    // );

    // info!("Test OK: Blocked customer");

    // assert_eq!(
    //     true,
    //     event_hub_common::licence::License::validate_license(
    //         &"043FBF659AFD5C2674B482D45809247EC389246BC4862E61370C67C04F4DBEDA75962387447744EB83DE6AB910930C8D1ADC0CA94670CB49D16A647196934725A41C9D5D11D0572771557DC9A9407C5F08937398184E01F81189CDE05B2C6A85E37A0A0E80141B1D16ABD603C0F340402FD7A2FAC781B01F62750EBF1847C409B467780EF8C4A57F2E05A60849"
    //             .to_string()
    //     )
    //     .is_err()
    // );

    // info!("Test OK: Old Version");
}

//////////////////////////////////////////////////

fn header(title: &str) {
    info!("\n{}", "-----------------------------------------------".white().on_blue());
    info!("        {}", title.white());
    info!("{}", "-----------------------------------------------".white().on_blue());
}

#[allow(dead_code)]
fn generate_new_secrets() {
    let (sk, pk) = generate_keypair();
    let (sk, pk) = (&sk.serialize(), &pk.serialize());

    let sk_hex_string = hex::encode_upper(sk);
    let pk_hex_string = hex::encode_upper(pk);
    info!("\nSK: {sk_hex_string}\nPK: {pk_hex_string}");
}
