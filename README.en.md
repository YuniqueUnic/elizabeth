# Elizabeth

Chinese: `README.md`

Elizabeth is a modern, room-centric file sharing and collaboration platform
built with Rust + Next.js.

## Quick Start (Docker, SQLite by default)

```bash
git clone https://github.com/YuniqueUnic/elizabeth.git
cd elizabeth

cp .env.docker .env
# Production: change JWT_SECRET (>= 32 chars)
${EDITOR:-nano} .env

docker compose up -d --build
```

Open:

- UI: `http://localhost:4001`
- OpenAPI: `http://localhost:4001/api/v1/docs`
- Health: `http://localhost:4001/api/v1/health`

## PostgreSQL (optional)

The backend uses `sqlx::AnyPool` and selects the driver based on `DATABASE_URL`.
Migrations are selected automatically:

- Source tree: SQLite → `crates/board/migrations`, PostgreSQL →
  `crates/board/migrations_pg`
- Docker runtime: `/app/migrations` and `/app/migrations_pg`

Enable PostgreSQL via the provided compose override:

```bash
docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build
```

Or connect to an external PostgreSQL by setting `DATABASE_URL=postgresql://...`
in `.env`.

## Configuration

- Docker: use `.env` for overrides (`JWT_SECRET`, `DATABASE_URL`,
  `DB_MAX_CONNECTIONS`, `DB_MIN_CONNECTIONS`, etc.)
- Config file: `docker/backend/config/backend.yaml` (note: YAML does not
  interpolate env vars; secrets should be injected via env)

## Development & Quality Gates (Rust)

```bash
cargo fmt --all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Or run `just verify` (see `justfile`).

## Docs

Start here: `docs/README.md`.

## License

See `LICENSE`.

Without prior written permission from the author, you may not modify the source
code, create derivative works, redistribute, deploy/host the software for others
or in production, or use it for any commercial purpose.

Open an issue if you need permission.
