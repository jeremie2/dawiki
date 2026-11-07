# dawiki

This project aims to explore ways to add wiki functionality to a website. It is not necessarily a wiki framework.

> ⚠️ This is a work in progress; use in a production environment is not recommended.

# Prerequisites

1. Install **Rust**.
2. Install **sqlx-cli** for database migrations via **cargo** :
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

# First configuration

1. Copy the dot env example file into the real `.env` choosing a proper name for the database:
   ```bash
   cp .env.example .env
   ```
2. Create the local database e apply the migrations:
   ```bash
   cargo sqlx database setup
   ```
3. Run the server:
   ```bash
   cargo run
   ```
