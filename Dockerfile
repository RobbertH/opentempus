# ---- web app ---------------------------------------------------------------
FROM node:22-alpine AS web
WORKDIR /src/web
RUN corepack enable
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

# ---- server ----------------------------------------------------------------
FROM rust:1.94-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY server/ ./server/
COPY --from=web /src/web/dist ./web/dist
RUN cargo build --release --locked

# ---- runtime ---------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home opentempus
COPY --from=server /src/target/release/opentempus /usr/local/bin/opentempus
USER opentempus
ENV OPENTEMPUS_LISTEN=0.0.0.0:8080
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s CMD curl -fsS http://127.0.0.1:8080/api/v1/health || exit 1
ENTRYPOINT ["/usr/local/bin/opentempus"]
