use clap::{Parser, Subcommand};
use colored::Colorize;
use directories::ProjectDirs;
use env_logger::Env;
use log::error;
use sqlx::SqlitePool;
use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;
use std::{env, process};
use walkdir::WalkDir;
use zip::write::FileOptions;

mod applications;
mod customers;
mod db;
mod license;

/// CLI definition
#[derive(Parser)]
#[command(name = "rustlock-admin")]
#[command(about = "Rustlock Interactive license & customer manager", long_about = None,)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new record
    Add {
        #[command(subcommand)]
        entity: AddEntity,
    },
    /// Show existing records
    Show {
        #[command(subcommand)]
        entity: ShowEntity,
    },
    /// Issue a new license
    Issue,
    /// Export database to a ZIP
    Backup,
    /// Validate a provided license string
    Validate,
    /// Update an existing record
    Update {
        #[command(subcommand)]
        entity: UpdateEntity,
    },
}

#[derive(Subcommand)]
enum AddEntity {
    /// Add a new customer (interactive)
    Customer,
    /// Add a new application (interactive)
    Application,
}

#[derive(Subcommand)]
enum ShowEntity {
    /// List all customers
    Customers,
    /// List all applications
    Applications {
        /// Show information about the application
        #[arg(long)]
        config: bool,
    },
    Licenses,
}

#[derive(Subcommand)]
enum UpdateEntity {
    /// Edit an existing customer’s fields
    Customer,
    Application,
}

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() {
    println!();
    println!("{}", " =====================================".green().bold());
    println!("{}", " |       RustLock Admin App          |".green().bold());
    println!("{}", " =====================================".green().bold());
    println!();

    // Initialize logging (default to "info")
    env_logger::Builder::from_env(Env::default().default_filter_or("info,sqlx=INFO,rustlock_core=TRACE")).init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Determine & prepare data directory
    let Some(proj_dirs) = ProjectDirs::from("", "Enlighten Systems", "rustlock-admin") else {
        error!("Cannot determine data directory via ProjectDirs");
        process::exit(1);
    };

    let data_dir = proj_dirs.data_dir();
    if !data_dir.exists() {
        if let Err(e) = create_dir_all(data_dir) {
            error!("Failed to create data directory {}: {e}", data_dir.display());
            process::exit(1);
        }
    }

    // Construct a SQLite URL. Sqlx expects "sqlite://<absolute_path>"
    let db_path = data_dir.join("rustlock.db");
    let db_url = format!("sqlite://{}", db_path.display());

    if !db_path.exists() {
        if let Err(e) = File::create(&db_path) {
            error!("Failed to create SQLite file {}: {e}", db_path.display());
            process::exit(1);
        }
    }

    // Connect via SQLx
    let pool = match SqlitePool::connect(&db_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to open SQLite via SQLx at {}: {e}", db_path.display());
            process::exit(1);
        }
    };

    // Ensure tables exist
    if let Err(e) = db::initialize_schema(&pool).await {
        error!("Failed to initialize database schema: {e}");
        process::exit(1);
    }

    // Dispatch on subcommands
    match cli.command {
        Commands::Add { entity } => match entity {
            AddEntity::Customer => {
                if let Err(e) = customers::add_customer_wizard(&pool).await {
                    error!("Error in add-customer flow: {e}");
                    process::exit(1);
                }
            }
            AddEntity::Application => {
                if let Err(e) = applications::add_application_wizard(&pool).await {
                    error!("Error in add-application flow: {e}");
                    process::exit(1);
                }
            }
        },
        Commands::Show { entity } => match entity {
            ShowEntity::Customers => {
                if let Err(e) = customers::show_customers(&pool).await {
                    error!("Failed to show customers: {e}");
                    process::exit(1);
                }
            }
            ShowEntity::Applications { config } => {
                if config {
                    if let Err(e) = applications::show_application_config(&pool).await {
                        error!("Failed to show applications config: {e}");
                        process::exit(1);
                    }
                } else if let Err(e) = applications::show_applications(&pool).await {
                    error!("Failed to show applications: {e}");
                    process::exit(1);
                }
            }
            ShowEntity::Licenses => {
                if let Err(e) = license::show_licenses(&pool).await {
                    error!("Failed to show licenses: {e}");
                    process::exit(1);
                }
            }
        },
        Commands::Issue => {
            if let Err(e) = license::issue_license_wizard(&pool).await {
                error!("Error in issue-license flow: {e}");
                process::exit(1);
            }
        }
        Commands::Backup => {
            if let Err(e) = backup_database(data_dir) {
                error!("Backup failed: {e}");
                process::exit(1);
            }
        }
        Commands::Validate => {
            if let Err(e) = license::validate_license_wizard(&pool).await {
                error!("Error in validate-license flow: {e}");
                process::exit(1);
            }
        }
        Commands::Update { entity } => match entity {
            UpdateEntity::Customer => {
                if let Err(e) = customers::update_customer_wizard(&pool).await {
                    error!("Error in update-customer flow: {e}");
                    process::exit(1);
                }
            }
            UpdateEntity::Application => {
                if let Err(e) = applications::update_application_wizard(&pool).await {
                    error!("Error in update-application flow: {e}");
                    process::exit(1);
                }
            }
        },
    }
}

/// Copy the `SQLite` database file (and anything else in `data_dir`) into a ZIP
fn backup_database(data_dir: &Path) -> io::Result<()> {
    // Create a timestamped backup ZIP in the current working directory
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_file_name = format!("rustlock-backup-{timestamp}.zip");
    let backup_path = env::current_dir()?.join(&backup_file_name);

    // Create the ZIP file (this can fail with an io::Error)
    let file = File::create(&backup_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated).unix_permissions(0o600);

    // Recursively walk data_dir and add every file
    for entry in WalkDir::new(data_dir) {
        let entry = entry.map_err(io::Error::other)?;
        let path = entry.path();
        if path.is_file() {
            // Derive a relative path inside the ZIP
            let rel_path = path.strip_prefix(data_dir).unwrap();

            // Read file contents into a buffer
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            io::copy(&mut f, &mut buffer)?;

            // Start a new file entry in the ZIP
            zip.start_file(rel_path.to_string_lossy(), options).map_err(io::Error::other)?;

            // Write the file’s bytes into the ZIP
            zip.write_all(&buffer)?;
        }
    }

    // Finish writing the ZIP (returns a zip::result::ZipError if something went wrong)
    zip.finish().map_err(io::Error::other)?;

    println!("✅ Backup created at: {}", backup_path.display());
    Ok(())
}
