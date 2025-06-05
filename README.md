# RustLock

RustLock provides hardware-locked licensing for Rust applications.
Licenses are tied to a specific machine and verified locally at runtime. All validation uses secure public/private key encryption so your apps run entirely offline.

## Why RustLock?

Many commercial applications require device‑bound licensing to prevent
unauthorized distribution. RustLock generates a secure fingerprint from key
hardware components and validates encrypted licenses so that only authorized
machines can run your software.

## Overview

1. **Generate a fingerprint** – Your application calls `get_system_fingerprint()`
   and sends the resulting string to your licensing server.
2. **Issue a license** – Use the `rustlock-admin` CLI to create customers,
   manage applications and produce license strings for each fingerprint.
3. **Validate on startup** – Ship the license string with your application and
   call `validate_license()` to ensure the machine and version match.

### Using `rustlock-core`

```rust
use rustlock_core::RustLock;

let license_public_key = "<public key>".to_string();
let blocked_customers = vec![9999];
let version = env!("CARGO_PKG_VERSION").to_string();
let machine_key = "<machine key>".to_string();
let info_private_key = "<info key>".to_string();

let lock = RustLock::new(
    license_public_key,
    blocked_customers,
    version,
    machine_key,
    info_private_key,
).unwrap();

// obtain the hardware fingerprint
let fingerprint = lock.get_system_fingerprint().unwrap();
// send `fingerprint` to your vendor and receive a license string

// later validate the license
let license = "<issued license>";
let license_info = lock.validate_license(&license).unwrap();
```

### Using `rustlock-admin`

`rustlock-admin` is an interactive CLI for managing applications, customers and
licenses. All records are stored in a local SQLite database.

```
rustlock-admin <COMMAND>
```

Common commands:

- `add customer` – create a customer record.
- `add application` – register an application and generate its keys.
- `show customers` – list all customers.
- `show applications [--config]` – list applications or dump configuration
  details.
- `show licenses` – display licenses for a selected application and customer.
- `issue` – generate a license for a given fingerprint.
- `validate` – check a license string.
- `update customer` – modify a customer record.
- `update application` – modify application details.
- `backup` – export the database as a ZIP archive.

Each command guides you through the required steps to issue and maintain
licenses.

## Getting Started

1. Run `rustlock-admin add application` to create your app entry and keys.
2. Run `rustlock-admin add customer` to register a customer.
3. Integrate `rustlock-core` in your application to collect the fingerprint.
4. Issue a license with `rustlock-admin issue`.
5. Distribute the license string and validate it on application startup.

Contribution and comments welcome!
