# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:alpine AS builder
WORKDIR /build

# musl-dev: C runtime headers for Alpine's musl toolchain.
# sqlite-dev: libsqlite3 headers/lib required by sqlx at compile time.
RUN apk add --no-cache musl-dev sqlite-dev

COPY . .
RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM alpine:3

# ca-certificates: trusted roots for outbound HTTPS monitoring (reqwest/rustls).
# libcap: provides setcap to grant raw-socket capabilities to the binary.
# sqlite-libs: dynamic libsqlite3 linked by the binary at runtime.
RUN apk add --no-cache ca-certificates libcap sqlite-libs && \
    adduser -D -H -s /sbin/nologin appuser

COPY --from=builder /build/target/release/rusty-pingus /usr/local/bin/rusty-pingus

# Grant raw-socket capabilities so ICMP ping and traceroute probes work without
# running as root. The container runtime must ALSO pass --cap-add=NET_RAW at
# startup (compose/Unraid handle this) — Linux drops file caps from the
# permitted set unless the runtime capability is also granted.
RUN setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/rusty-pingus

USER appuser

ENV BIND_PORT=8080

# Shell form allows ${BIND_PORT} expansion at container start.
# Mount /config (config.toml + monitors.toml) and /data (SQLite DB) as volumes.
CMD /usr/local/bin/rusty-pingus \
    --bind "0.0.0.0:${BIND_PORT}" \
    --config /config/config.toml \
    --monitors /config/monitors.toml \
    --db /data/rusty-pingus.db
