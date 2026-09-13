# Contributing

* `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass. CI enforces them.
* `cd web && pnpm build` type-checks the web app.
* Migrations live in `server/migrations` and are applied at startup. Never edit a migration that has been merged; add a new one.
* New inbound connectors implement `sync::connector::Connector` and are wired in `connector_for`. New write-back kinds add a branch in `sync::push::push_target`.
* For CalDAV work, Radicale (`pip install radicale`) is a convenient local server; see `docs/ARCHITECTURE.md`.
* Anything that touches what a share exposes needs a unit test in `sharing.rs` or `ics/`.
* Keep the "one binary + Postgres" rule (see `docs/adr/0001-postgres-as-queue.md`).
* Provider setup guides live in `web/src/help/guides.ts`, drawn with the SVG kit in `web/src/help/Shot.tsx`. They are schematics, not screenshots: do not paste captures of other companies' interfaces. When a provider moves a button, fix the label there and keep `docs/CONNECTING.md` in step.
