use std::error::Error;

use chrono::{Datelike, Utc};
use dialoguer::{Input, Select, theme::ColorfulTheme};
use ecies::{decrypt, encrypt};
use log::{error, info};
use rustlock_core::{RustLock, license::License, sysinfo::SysInfo};
use sqlx::{Pool, Row, Sqlite};
use version_compare::Version;

#[allow(clippy::too_many_lines)]
/// Interactive wizard to issue a license
pub async fn issue_license_wizard(pool: &Pool<Sqlite>) -> Result<(), Box<dyn Error>> {
    let theme = ColorfulTheme::default();

    // 1) Select an application
    let apps = crate::db::fetch_applications(pool).await?;
    if apps.is_empty() {
        println!("⚠️  No applications found. Please add one first.");
        return Ok(());
    }
    let app_choices: Vec<String> = apps.iter().map(|app| format!("ID {} – {}", app.id, app.name)).collect();
    let app_selection = Select::with_theme(&theme).with_prompt("Select application to issue license for").default(0).items(&app_choices).interact().unwrap();
    let chosen_app = &apps[app_selection];

    // 2) Select a customer
    let customers = crate::db::fetch_customers(pool).await?;
    if customers.is_empty() {
        println!("⚠️  No customers found. Please add one first.");
        return Ok(());
    }
    let cust_choices: Vec<String> = customers.iter().map(|c| format!("ID {} – {}", c.id, c.name)).collect();
    let cust_selection = Select::with_theme(&theme).with_prompt("Select customer to link license to").default(0).items(&cust_choices).interact().unwrap();
    let chosen_cust = &customers[cust_selection];

    // 3) HWID input
    let hwid: String = Input::with_theme(&theme).with_prompt("Enter HWID string").interact_text().unwrap();

    // 4) Support years (default = 1)
    let support_years: i32 = Input::with_theme(&theme).with_prompt("Support years").default(1).interact_text().unwrap();

    let version: String = Input::with_theme(&theme)
        .with_prompt("License version (semver, e.g., 1.0.3)")
        .with_initial_text("1.0.0")
        .validate_with(|input: &String| -> Result<(), &str> {
            if Version::from(input).is_some() { Ok(()) } else { Err("Invalid version format; expected semver (e.g., 1.2.3)") }
        })
        .interact_text()?;

    // 5) For each non-null feature name on the application, ask Yes/No
    //    to include that feature in this license.
    let mut include_feature1 = false;
    let mut include_feature2 = false;
    let mut include_feature3 = false;
    let mut include_feature4 = false;
    let mut include_feature5 = false;

    if let Some(feat1_name) = &chosen_app.feature1 {
        let ans = Select::with_theme(&theme).with_prompt(format!("Include feature '{feat1_name}'?")).default(0).items(&["No", "Yes"]).interact().unwrap();
        include_feature1 = ans == 1;
    }
    if let Some(feat2_name) = &chosen_app.feature2 {
        let ans = Select::with_theme(&theme).with_prompt(format!("Include feature '{feat2_name}'?")).default(0).items(&["No", "Yes"]).interact().unwrap();
        include_feature2 = ans == 1;
    }
    if let Some(feat3_name) = &chosen_app.feature3 {
        let ans = Select::with_theme(&theme).with_prompt(format!("Include feature '{feat3_name}'?")).default(0).items(&["No", "Yes"]).interact().unwrap();
        include_feature3 = ans == 1;
    }
    if let Some(feat4_name) = &chosen_app.feature4 {
        let ans = Select::with_theme(&theme).with_prompt(format!("Include feature '{feat4_name}'?")).default(0).items(&["No", "Yes"]).interact().unwrap();
        include_feature4 = ans == 1;
    }
    if let Some(feat5_name) = &chosen_app.feature5 {
        let ans = Select::with_theme(&theme).with_prompt(format!("Include feature '{feat5_name}'?")).default(0).items(&["No", "Yes"]).interact().unwrap();
        include_feature5 = ans == 1;
    }

    let fingerprint = decode_hwinfo_from_string(&hwid, &chosen_app.info_public_key.clone()).unwrap();
    println!();
    info!("HW Info:\n{fingerprint:#?}");
    println!();

    let mut lic = License::default();

    let current_version = Version::from(&version).unwrap();
    // set max version
    lic.version = current_version.part(0).unwrap().to_string() + "." + &current_version.part(1).unwrap().to_string() + ".9999";

    lic.name.clone_from(&chosen_cust.name);

    let date = Utc::now();

    lic.customer = chosen_cust.id;
    lic.start_month = date.month();
    lic.start_year = date.year();

    lic.end_month = date.month();
    lic.end_year = date.year() + support_years;

    lic.c1 = fingerprint.o_hash;
    lic.c2 = fingerprint.c_hash;
    lic.c3 = fingerprint.s_hash;
    lic.c4 = fingerprint.n_hash;

    lic.f1 = include_feature1;
    lic.f2 = include_feature2;
    lic.f3 = include_feature3;
    lic.f4 = include_feature4;
    lic.f5 = include_feature5;

    let lic_pk = hex::decode(chosen_app.lic_private_key.clone()).unwrap();
    let msg = rmp_serde::to_vec(&lic).unwrap();

    let encrypted = encrypt(&lic_pk, &msg).unwrap();
    let encrypted_string = hex::encode_upper(encrypted);

    println!();
    info!("Generated License: {encrypted_string}");
    println!();

    let lock = RustLock::new(chosen_app.lic_public_key.clone(), chosen_app.blocked_customer_ids.clone(), version, chosen_app.machine_id_key.clone(), chosen_app.info_private_key.clone())?;

    let valid_lic = lock.read_license(&encrypted_string)?;

    println!();
    info!("License: {valid_lic:#?}");
    println!();

    // Insert into licenses
    sqlx::query(
        r"
        INSERT INTO licenses (
            hwid,
            support_years,
            customer_id,
            application_id,
            issued_license
        )
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
    )
    .bind(&hwid)
    .bind(support_years)
    .bind(chosen_cust.id)
    .bind(chosen_app.id)
    .bind(encrypted_string)
    .execute(pool)
    .await?;

    info!(
        "Issued new license for app {} to customer {} (features: {}, {}, {}, {}, {})",
        chosen_app.id, chosen_cust.id, include_feature1, include_feature2, include_feature3, include_feature4, include_feature5,
    );
    println!("✅ License record created.");
    Ok(())
}

/// Interactive wizard to validate a license
pub async fn validate_license_wizard(pool: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    // 1) Select application context
    let apps = crate::db::fetch_applications(pool).await?;
    if apps.is_empty() {
        println!("⚠️  No applications found.");
        return Ok(());
    }
    let app_choices: Vec<String> = apps.iter().map(|app| format!("ID {} – {}", app.id, app.name)).collect();
    let app_selection = Select::with_theme(&theme).with_prompt("Select application context for validation").default(0).items(&app_choices).interact().unwrap();
    let chosen_app = &apps[app_selection];

    // 2) Enter license string
    let lic_str: String = Input::with_theme(&theme).with_prompt("Paste license string to validate").interact_text().unwrap();

    let version_str: String = Input::with_theme(&theme).with_prompt("Enter app version validate").interact_text().unwrap();

    let lock = RustLock::new(chosen_app.lic_public_key.clone(), chosen_app.blocked_customer_ids.clone(), version_str, chosen_app.machine_id_key.clone(), chosen_app.info_private_key.clone())?;

    match lock.read_license(&lic_str) {
        Ok(_) => println!("✅ License string is VALID but not Validated."),
        Err(_) => println!("❌ License is INVALID."),
    }

    Ok(())
}

fn decode_hwinfo_from_string(input: &str, public_key: &str) -> Option<SysInfo> {
    // Customer has private, we have public
    let Ok(sk) = hex::decode(public_key) else {
        error!("Failed to Decode Public Key");
        return None;
    };

    let Ok(payload) = hex::decode(input) else {
        error!("Failed to Decode Input");
        return None;
    };

    let Ok(decrypted) = decrypt(&sk, &payload) else {
        error!("Failed to Decrypt HWInfo");
        return None;
    };

    rmp_serde::from_read::<&[u8], SysInfo>(&*decrypted).ok()
}

/// Show all licenses for a selected application and customer.
/// Since HWID and `issued_license` strings can be very long, each record is printed in full without a table.
pub async fn show_licenses(pool: &Pool<Sqlite>) -> Result<(), Box<dyn Error>> {
    let theme = ColorfulTheme::default();

    // 1) Select an application
    let apps = crate::db::fetch_applications(pool).await?;
    if apps.is_empty() {
        println!("⚠️  No applications found.");
        return Ok(());
    }
    let app_choices: Vec<String> = apps.iter().map(|app| format!("ID {} – {}", app.id, app.name)).collect();
    let app_selection = Select::with_theme(&theme).with_prompt("Select application to view licenses for").default(0).items(&app_choices).interact()?;
    let chosen_app = &apps[app_selection];

    // 2) Select a customer
    let customers = crate::db::fetch_customers(pool).await?;
    if customers.is_empty() {
        println!("⚠️  No customers found.");
        return Ok(());
    }
    let cust_choices: Vec<String> = customers.iter().map(|c| format!("ID {} – {}", c.id, c.name)).collect();
    let cust_selection = Select::with_theme(&theme).with_prompt("Select customer to view licenses for").default(0).items(&cust_choices).interact()?;
    let chosen_cust = &customers[cust_selection];

    // 3) Query licenses for the chosen app/customer
    let rows = sqlx::query(
        r"
        SELECT
          id,
          hwid,
          support_years,
          issued_license
        FROM licenses
        WHERE application_id = ?1
          AND customer_id = ?2
        ORDER BY id
        ",
    )
    .bind(chosen_app.id)
    .bind(chosen_cust.id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        println!("⚠️  No licenses found for application '{}' (ID {}) and customer '{}' (ID {}).", chosen_app.name, chosen_app.id, chosen_cust.name, chosen_cust.id);
        return Ok(());
    }

    println!();
    println!("—— Licenses for App '{}' (ID {}) and Customer '{}' (ID {}) ——————————", chosen_app.name, chosen_app.id, chosen_cust.name, chosen_cust.id);
    println!();
    println!("────────────────────────────────────────────────────────────────");

    // 4) Print each license record in full
    for row in &rows {
        let license_id: i64 = row.try_get("id")?;
        let hwid: String = row.try_get("hwid")?;
        let support_years: i32 = row.try_get("support_years")?;
        let issued_license: String = row.try_get("issued_license")?;

        println!("License ID       : {license_id}");
        println!("Support Years    : {support_years}");
        println!();
        println!("HWID             : {hwid}");
        println!();
        println!("Issued License   : {issued_license}");
        println!("────────────────────────────────────────────────────────────────");
    }

    info!("Displayed {} license(s) for app {} and customer {}.", rows.len(), chosen_app.id, chosen_cust.id);
    Ok(())
}
