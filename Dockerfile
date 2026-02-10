# ── Build stage (uses pre-cached dependencies from base image) ─────────
FROM resawod-base AS builder

COPY src/ src/
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/resawod-scheduler /usr/local/bin/resawod-scheduler

WORKDIR /app

EXPOSE 3009

ENTRYPOINT ["resawod-scheduler"]
CMD ["serve", "--config", "/app/config.toml"]
