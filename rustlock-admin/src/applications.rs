use std::error::Error;

use dialoguer::{Input, Select, theme::ColorfulTheme};
use ecies::utils::generate_keypair;
use log::info;
use rustlock_core::RustLock;
use serde_json::to_string as json_to_string;
use sqlx::{Pool, Row, Sqlite};

/// Prompt the user to select one application, then print all its key fields and feature names.
pub async fn show_application_config(pool: &Pool<Sqlite>) -> Result<(), Box<dyn std::error::Error>> {
    // 1) Fetch all columns of each application, including the five feature columns
    let rows = sqlx::query(
        r#"
        SELECT
          id,
          name,
          lic_public_key,
          lic_private_key,
          blocked_customer_ids,
          machine_id_key,
          info_public_key,
          info_private_key,
          feature1,
          feature2,
          feature3,
          feature4,
          feature5
        FROM applications
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        println!("⚠️  No applications found.");
        return Ok(());
    }

    // 2) Build a Vec<(id, name)> so we can prompt with “ID – Name”
    let mut choices = Vec::new();
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        choices.push(format!("ID {id} – {name}"));
    }

    // 3) Let the user pick one
    let selection = Select::with_theme(&ColorfulTheme::default()).with_prompt("Select an application to dump config for").default(0).items(&choices).interact()?;

    // 4) Extract the chosen row
    let chosen_row = &rows[selection];
    let id: i64 = chosen_row.try_get("id")?;
    let name: String = chosen_row.try_get("name")?;
    let lic_pub: String = chosen_row.try_get("lic_public_key")?;
    let machine_key: String = chosen_row.try_get("machine_id_key")?;
    let info_priv: String = chosen_row.try_get("info_private_key")?;

    // 5) Extract each feature (they may be NULL)
    let feature1: Option<String> = chosen_row.try_get("feature1")?;
    let feature2: Option<String> = chosen_row.try_get("feature2")?;
    let feature3: Option<String> = chosen_row.try_get("feature3")?;
    let feature4: Option<String> = chosen_row.try_get("feature4")?;
    let feature5: Option<String> = chosen_row.try_get("feature5")?;

    // 6) Print everything out, including features
    println!();
    println!("—— Application Config for {}  - ID {} —————————————", name, id);
    println!();

    println!("// Copy and paste the following into your application:");
    println!("let license_public_key = \"{}\".to_string();", lic_pub);
    println!("let machine_key = \"{}\".to_string();", machine_key);
    println!("let info_private_key = \"{}\".to_string(); // Info encrypted on client side", info_priv);
    println!();
    println!("let blocked_customers = vec![9999]; // Example Block Customer 9999");
    println!("let version = env!(\"CARGO_PKG_VERSION\").to_string();");
    println!();

    // Print each feature; if None, show as empty string
    println!("// Feature names (empty if none):");
    println!("let feature1 = \"{}\".to_string();", feature1.clone().unwrap_or_default());
    println!("let feature2 = \"{}\".to_string();", feature2.clone().unwrap_or_default());
    println!("let feature3 = \"{}\".to_string();", feature3.clone().unwrap_or_default());
    println!("let feature4 = \"{}\".to_string();", feature4.clone().unwrap_or_default());
    println!("let feature5 = \"{}\".to_string();", feature5.clone().unwrap_or_default());
    println!();

    println!("let lock = RustLock::new(");
    println!("    license_public_key,");
    println!("    blocked_customers,");
    println!("    version,");
    println!("    machine_key,");
    println!("    info_private_key");
    println!(");");
    println!();

    println!("——————————————————————————————————————————————");

    Ok(())
}

/// Show all applications, displaying each app’s name,
/// how many distinct customers have licenses for it, and how many licenses exist.
pub async fn show_applications(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    // Aggregate query: count total licenses and distinct customers per application
    let rows = sqlx::query(
        r#"
        SELECT 
            a.id,
            a.name,
            COUNT(l.id) AS license_count,
            COUNT(DISTINCT l.customer_id) AS customer_count
        FROM applications a
        LEFT JOIN licenses l
          ON a.id = l.application_id
        GROUP BY a.id, a.name
        ORDER BY a.id
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("{}", "-".repeat(sixty_four()));
    println!("{:<6} | {:<20} | {:<15} | {:<15}", "ID", "Name", "# Customers", "# Licenses");
    println!("{}", "-".repeat(sixty_four()));

    for row in rows {
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let license_count: i64 = row.try_get("license_count")?;
        let customer_count: i64 = row.try_get("customer_count")?;

        println!("{:<6} | {:<20} | {:<15} | {:<15}", id, name, customer_count, license_count);
    }

    println!("{}", "-".repeat(sixty_four()));
    Ok(())
}

fn sixty_four() -> usize {
    64
}

/// Allows editing an existing application’s fields (name, keys, features, etc.)
pub async fn update_application_wizard(pool: &Pool<Sqlite>) -> Result<(), Box<dyn Error>> {
    let theme = ColorfulTheme::default();

    // 1) Fetch all applications (the App struct must now include feature1..feature5)
    let apps = crate::db::fetch_applications(pool).await?;
    if apps.is_empty() {
        println!("⚠️  No applications found. Please add one first.");
        return Ok(());
    }

    // 2) Let the user pick which application to update
    let choices: Vec<String> = apps.iter().map(|app| format!("ID {} – {}", app.id, app.name)).collect();
    let selection = Select::with_theme(&theme).with_prompt("Select an application to update").default(0).items(&choices).interact()?;
    let app = &apps[selection];

    // 3) Prompt for each field, prefilled with current values.
    //    Pressing Enter leaves the field unchanged.

    // a) Name
    let new_name: String = Input::with_theme(&theme).with_prompt("Application name").with_initial_text(app.name.clone()).interact_text()?;

    // b) lic_public_key
    let new_lic_pub: String = Input::with_theme(&theme).with_prompt("License public key").with_initial_text(app.lic_public_key.clone()).interact_text()?;

    // c) lic_private_key
    let new_lic_priv: String = Input::with_theme(&theme).with_prompt("License private key").with_initial_text(app.lic_private_key.clone()).interact_text()?;

    // d) machine_id_key
    let new_machine_key: String = Input::with_theme(&theme).with_prompt("Machine ID key").with_initial_text(app.machine_id_key.clone()).interact_text()?;

    // e) info_public_key
    let new_info_pub: String = Input::with_theme(&theme).with_prompt("Info public key").with_initial_text(app.info_public_key.clone()).interact_text()?;

    // f) info_private_key
    let new_info_priv: String = Input::with_theme(&theme).with_prompt("Info private key").with_initial_text(app.info_private_key.clone()).interact_text()?;

    // g) feature1
    let new_feature1: String = Input::with_theme(&theme)
        .with_prompt("Feature1 name (leave blank to keep none)")
        .with_initial_text(app.feature1.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // h) feature2
    let new_feature2: String = Input::with_theme(&theme)
        .with_prompt("Feature2 name (leave blank to keep none)")
        .with_initial_text(app.feature2.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // i) feature3
    let new_feature3: String = Input::with_theme(&theme)
        .with_prompt("Feature3 name (leave blank to keep none)")
        .with_initial_text(app.feature3.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // j) feature4
    let new_feature4: String = Input::with_theme(&theme)
        .with_prompt("Feature4 name (leave blank to keep none)")
        .with_initial_text(app.feature4.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // k) feature5
    let new_feature5: String = Input::with_theme(&theme)
        .with_prompt("Feature5 name (leave blank to keep none)")
        .with_initial_text(app.feature5.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    // Convert empty strings into None so the column becomes NULL
    let f1_opt: Option<String> = if new_feature1.trim().is_empty() { None } else { Some(new_feature1.clone()) };
    let f2_opt: Option<String> = if new_feature2.trim().is_empty() { None } else { Some(new_feature2.clone()) };
    let f3_opt: Option<String> = if new_feature3.trim().is_empty() { None } else { Some(new_feature3.clone()) };
    let f4_opt: Option<String> = if new_feature4.trim().is_empty() { None } else { Some(new_feature4.clone()) };
    let f5_opt: Option<String> = if new_feature5.trim().is_empty() { None } else { Some(new_feature5.clone()) };

    // 4) Run the UPDATE statement (now including feature1..feature5)
    sqlx::query(
        r#"
        UPDATE applications
        SET
          name                = ?1,
          lic_public_key      = ?2,
          lic_private_key     = ?3,
          machine_id_key      = ?4,
          info_public_key     = ?5,
          info_private_key    = ?6,
          feature1            = ?7,
          feature2            = ?8,
          feature3            = ?9,
          feature4            = ?10,
          feature5            = ?11
        WHERE id = ?12
        "#,
    )
    .bind(&new_name)
    .bind(&new_lic_pub)
    .bind(&new_lic_priv)
    .bind(&new_machine_key)
    .bind(&new_info_pub)
    .bind(&new_info_priv)
    .bind(f1_opt)
    .bind(f2_opt)
    .bind(f3_opt)
    .bind(f4_opt)
    .bind(f5_opt)
    .bind(app.id)
    .execute(pool)
    .await?;

    info!("Application ID {} updated.", app.id);
    println!("✅ Application updated successfully!");
    Ok(())
}

fn generate_new_secrets() -> (String, String) {
    let (sk, pk) = generate_keypair();
    let (sk, pk) = (&sk.serialize(), &pk.serialize());

    let sk_hex_string = hex::encode_upper(sk);
    let pk_hex_string = hex::encode_upper(pk);

    (sk_hex_string, pk_hex_string)
}

/// Interactive wizard to add a new application (including five optional features)
pub async fn add_application_wizard(pool: &Pool<Sqlite>) -> Result<(), Box<dyn Error>> {
    let theme = ColorfulTheme::default();

    let name: String = Input::with_theme(&theme).with_prompt("Application name").interact_text()?;

    let (lic_public_key, lic_private_key) = generate_new_secrets();
    let (info_public_key, info_private_key) = generate_new_secrets();
    let (_, machine_id_key) = generate_new_secrets();

    // blocked_customer_ids: start example with [9999]
    let blocked_customer_ids: Vec<u16> = vec![9999];
    let blocked_ids_json = json_to_string(&blocked_customer_ids).unwrap();

    let lock = RustLock::new(lic_public_key.clone(), blocked_customer_ids.clone(), "0.0.1".to_string(), machine_id_key.clone(), info_private_key.clone());

    let fingerprint = lock.get_system_fingerprint();

    // Prompt for feature1..feature5 (each may be left blank)
    let feature1: String = Input::with_theme(&theme).with_prompt("Feature1 name (leave blank if none)").allow_empty(true).interact_text()?;
    let feature2: String = Input::with_theme(&theme).with_prompt("Feature2 name (leave blank if none)").allow_empty(true).interact_text()?;
    let feature3: String = Input::with_theme(&theme).with_prompt("Feature3 name (leave blank if none)").allow_empty(true).interact_text()?;
    let feature4: String = Input::with_theme(&theme).with_prompt("Feature4 name (leave blank if none)").allow_empty(true).interact_text()?;
    let feature5: String = Input::with_theme(&theme).with_prompt("Feature5 name (leave blank if none)").allow_empty(true).interact_text()?;

    // Convert empty strings into None so the column becomes NULL
    let f1_opt: Option<String> = if feature1.trim().is_empty() { None } else { Some(feature1.clone()) };
    let f2_opt: Option<String> = if feature2.trim().is_empty() { None } else { Some(feature2.clone()) };
    let f3_opt: Option<String> = if feature3.trim().is_empty() { None } else { Some(feature3.clone()) };
    let f4_opt: Option<String> = if feature4.trim().is_empty() { None } else { Some(feature4.clone()) };
    let f5_opt: Option<String> = if feature5.trim().is_empty() { None } else { Some(feature5.clone()) };

    // Show all stub values & ask for confirmation
    info!("Generated the following stub fields for the new application:");
    println!("• lic_public_key: {}", lic_public_key);
    println!("• info_public_key: {}", info_public_key);
    println!("• machine_id_key: {}", machine_id_key);
    println!("• blocked_customer_ids: {:?}", blocked_customer_ids);
    println!("• fingerprint test: {fingerprint}");
    println!();

    // Also display the entered feature names (or indicate "none")
    println!("• feature1: {}", f1_opt.clone().unwrap_or_else(|| "<none>".to_string()));
    println!("• feature2: {}", f2_opt.clone().unwrap_or_else(|| "<none>".to_string()));
    println!("• feature3: {}", f3_opt.clone().unwrap_or_else(|| "<none>".to_string()));
    println!("• feature4: {}", f4_opt.clone().unwrap_or_else(|| "<none>".to_string()));
    println!("• feature5: {}", f5_opt.clone().unwrap_or_else(|| "<none>".to_string()));
    println!();

    let choices = vec!["Save application", "Cancel"];
    let selection = Select::with_theme(&theme).with_prompt("Would you like to save this application?").default(0).items(&choices).interact().unwrap();

    if selection != 0 {
        info!("Aborted—no application was created.");
        return Ok(());
    }

    // Insert into DB, including feature1..feature5
    sqlx::query(
        r#"
        INSERT INTO applications (
            name,
            lic_public_key,
            lic_private_key,
            blocked_customer_ids,
            machine_id_key,
            info_public_key,
            info_private_key,
            feature1,
            feature2,
            feature3,
            feature4,
            feature5
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
    )
    .bind(&name)
    .bind(&lic_public_key)
    .bind(&lic_private_key)
    .bind(&blocked_ids_json)
    .bind(&machine_id_key)
    .bind(&info_public_key)
    .bind(&info_private_key)
    .bind(f1_opt)
    .bind(f2_opt)
    .bind(f3_opt)
    .bind(f4_opt)
    .bind(f5_opt)
    .execute(pool)
    .await?;

    info!("✅ Application created!");
    Ok(())
}
