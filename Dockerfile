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
# su-exec: tiny gosu-equivalent used by the entrypoint to drop root → PUID:PGID.
RUN apk add --no-cache ca-certificates libcap sqlite-libs su-exec

COPY --from=builder /build/target/release/rusty-pingus /usr/local/bin/rusty-pingus
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Grant raw-socket capabilities so ICMP ping and traceroute probes work without
# running as root. The container runtime must ALSO pass --cap-add=NET_RAW at
# startup (compose/Unraid handle this) — Linux drops file caps from the
# permitted set unless the runtime capability is also granted.
RUN setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/rusty-pingus

# Defaults match Unraid's nobody:users (99:100), which owns /mnt/user/appdata;
# override PUID/PGID for other hosts. The container starts as root so the
# entrypoint can chown the mounts, then drops to PUID:PGID via su-exec.
ENV BIND_PORT=8080 \
    PUID=99 \
    PGID=100

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Shell form (via the entrypoint's `sh -c`) allows ${BIND_PORT} expansion.
# Mount /config (config.toml + monitors.toml) and /data (SQLite DB) as volumes.
CMD /usr/local/bin/rusty-pingus \
    --bind "0.0.0.0:${BIND_PORT}" \
    --config /config/config.toml \
    --monitors /config/monitors.toml \
    --db /data/rusty-pingus.db
