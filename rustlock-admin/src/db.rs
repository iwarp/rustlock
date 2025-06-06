use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

#[derive(Debug)]
pub struct Customer {
    pub id: u16,
    pub name: String,
    pub contact_email: String,
    pub mobile: String,
}

/// Application table representation
#[derive(Debug, Serialize, Deserialize)]
pub struct Application {
    pub id: i64,
    pub name: String,
    pub lic_public_key: String,
    pub lic_private_key: String,
    pub blocked_customer_ids: Vec<u16>,
    pub machine_id_key: String,
    pub info_public_key: String,
    pub info_private_key: String,
    pub feature1: Option<String>,
    pub feature2: Option<String>,
    pub feature3: Option<String>,
    pub feature4: Option<String>,
    pub feature5: Option<String>,
}

/// Create tables if they do not exist yet
pub async fn initialize_schema(pool: &Pool<Sqlite>) -> sqlx::Result<()> {
    // customers
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS customers (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            name          TEXT NOT NULL,
            contact_email TEXT NOT NULL,
            mobile        TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // applications
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS applications (
            id                    INTEGER PRIMARY KEY AUTOINCREMENT,
            name                  TEXT NOT NULL,
            lic_public_key        TEXT NOT NULL,
            lic_private_key       TEXT NOT NULL,
            blocked_customer_ids  TEXT NOT NULL,
            machine_id_key        TEXT NOT NULL,
            info_public_key       TEXT NOT NULL,
            info_private_key      TEXT NOT NULL,
            feature1              TEXT,
            feature2              TEXT,
            feature3              TEXT,
            feature4              TEXT,
            feature5              TEXT
        )",
    )
    .execute(pool)
    .await?;

    // licenses (placeholder)
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS licenses (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            hwid            TEXT NOT NULL,
            support_years   INTEGER NOT NULL,
            customer_id     INTEGER NOT NULL,
            application_id  INTEGER NOT NULL,
            issued_license  TEXT, 
            FOREIGN KEY(customer_id) REFERENCES customers(id),
            FOREIGN KEY(application_id) REFERENCES applications(id)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch all customers from the database
pub async fn fetch_customers(pool: &Pool<Sqlite>) -> sqlx::Result<Vec<Customer>> {
    let rows = sqlx::query("SELECT id, name, contact_email, mobile FROM customers").fetch_all(pool).await?;

    let mut list = Vec::new();
    for row in rows {
        list.push(Customer {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            contact_email: row.try_get("contact_email")?,
            mobile: row.try_get("mobile")?,
        });
    }
    Ok(list)
}

/// Fetch all applications from the database
pub async fn fetch_applications(pool: &Pool<Sqlite>) -> sqlx::Result<Vec<Application>> {
    let rows = sqlx::query(
        r"
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
        ",
    )
    .fetch_all(pool)
    .await?;

    let mut list = Vec::new();
    for row in rows {
        let blocked_json: String = row.try_get("blocked_customer_ids")?;
        let blocked_ids: Vec<u16> = serde_json::from_str(&blocked_json).unwrap_or_default();
        list.push(Application {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            lic_public_key: row.try_get("lic_public_key")?,
            lic_private_key: row.try_get("lic_private_key")?,
            blocked_customer_ids: blocked_ids,
            machine_id_key: row.try_get("machine_id_key")?,
            info_public_key: row.try_get("info_public_key")?,
            info_private_key: row.try_get("info_private_key")?,

            feature1: row.try_get("feature1")?,
            feature2: row.try_get("feature2")?,
            feature3: row.try_get("feature3")?,
            feature4: row.try_get("feature4")?,
            feature5: row.try_get("feature5")?,
        });
    }
    Ok(list)
}
