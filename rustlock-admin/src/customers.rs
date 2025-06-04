use dialoguer::{Input, Select, theme::ColorfulTheme};
use log::info;
use sqlx::{Pool, Row, Sqlite};

/// Show all customers in a simple table
pub async fn show_customers(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    let rows = sqlx::query("SELECT id, name, contact_email, mobile FROM customers").fetch_all(pool).await?;

    println!("{}", "-".repeat(80));
    println!("{:<6} | {:<20} | {:<30} | {:<15}", "ID", "Name", "Email", "Mobile");
    println!("{}", "-".repeat(80));

    for row in rows {
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let email: String = row.try_get("contact_email")?;
        let mobile: String = row.try_get("mobile")?;
        println!("{id:<6} | {name:<20} | {email:<30} | {mobile:<15}");
    }

    Ok(())
}

/// Interactive wizard to add a new customer
pub async fn add_customer_wizard(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    let theme = ColorfulTheme::default();

    // 1) Prompt for customer name
    let name: String = Input::with_theme(&theme).with_prompt("Customer name").interact_text().unwrap();

    // 2) Prompt for contact email (simple validation)
    let contact_email: String = Input::with_theme(&theme)
        .with_prompt("Contact email")
        .validate_with(|input: &String| if input.contains('@') { Ok(()) } else { Err("Must be a valid email address") })
        .interact_text()
        .unwrap();

    // 3) Prompt for mobile
    let mobile: String = Input::with_theme(&theme).with_prompt("Mobile number").interact_text().unwrap();

    // 4) Insert into database
    sqlx::query(
        r"
        INSERT INTO customers (name, contact_email, mobile)
        VALUES (?1, ?2, ?3)
        ",
    )
    .bind(&name)
    .bind(&contact_email)
    .bind(&mobile)
    .execute(pool)
    .await?;

    info!("New customer added successfully.");
    info!("✅ Customer added!");
    Ok(())
}

/// Interactive wizard to update an existing customer
pub async fn update_customer_wizard(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    let theme = ColorfulTheme::default();

    // 1) Fetch all customers
    let customers = crate::db::fetch_customers(pool).await?;
    if customers.is_empty() {
        println!("⚠️  No customers found. Please add one first.");
        return Ok(());
    }

    // 2) Prompt the user to select which customer to update
    let choices: Vec<String> = customers.iter().map(|c| format!("ID {} – {}", c.id, c.name)).collect();
    let selection = Select::with_theme(&theme).with_prompt("Select a customer to update").default(0).items(&choices).interact().unwrap();
    let cust = &customers[selection];

    // 3) Prompt for each field, prefilled with current values.
    //    If the user just hits “Enter,” the default (current value) is used.

    // Name
    let new_name: String = Input::with_theme(&theme).with_prompt("Customer name").with_initial_text(cust.name.clone()).interact_text().unwrap();

    // Contact email
    let new_email: String = Input::with_theme(&theme)
        .with_prompt("Contact email")
        .with_initial_text(cust.contact_email.clone())
        .validate_with(|input: &String| if input.contains('@') { Ok(()) } else { Err("Must be a valid email address") })
        .interact_text()
        .unwrap();

    // Mobile
    let new_mobile: String = Input::with_theme(&theme).with_prompt("Mobile number").with_initial_text(cust.mobile.clone()).interact_text().unwrap();

    // 4) Run the UPDATE query
    sqlx::query(
        r"
        UPDATE customers
        SET name = ?1,
            contact_email = ?2,
            mobile = ?3
        WHERE id = ?4
        ",
    )
    .bind(&new_name)
    .bind(&new_email)
    .bind(&new_mobile)
    .bind(cust.id)
    .execute(pool)
    .await?;

    info!("Customer ID {} updated.", cust.id);
    println!("✅ Customer updated successfully!");
    Ok(())
}
