# dawiki

This project aims to explore ways to add wiki functionality to a website. It is not necessarily a wiki framework.

> ⚠️ This is a work in progress; use in a production environment is not recommended.

> 📝 Sorry if the user interface is in Italian... This because if the project will reach good results it could be used in an Italian website and it's not intended to be a real *framework*. The code (function names, variables, comments, etc..) is all in English.

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

# Usage

- To create a new page go to `http://127.0.0.1:3000/~nuova`. Write some markdown a click on **Salva pagina**.
- To update a page click on **Modifica** or into the address bar `http://127.0.0.1:3000/~modifica/name_of_existing_page`.
