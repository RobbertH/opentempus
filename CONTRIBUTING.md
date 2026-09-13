# Contributing

* `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass. CI enforces them.
* `cd web && pnpm build` type-checks the web app.
* Migrations live in `server/migrations` and are applied at startup. Never edit a migration that has been merged; add a new one.
* New inbound connectors implement `sync::connector::Connector` and are wired in `connector_for`.
* Anything that touches what a share exposes needs a unit test in `sharing.rs` or `ics/`.
* Keep the "one binary + Postgres" rule (see `docs/adr/0001-postgres-as-queue.md`).
