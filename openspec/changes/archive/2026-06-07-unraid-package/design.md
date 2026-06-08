## Context

rusty-pingus is a Rust/Axum binary that embeds all frontend assets via `rust-embed`. It takes CLI flags (`--config`, `--bind`, `--db`, `--monitors`) rather than environment variables, and writes a SQLite database to `./data/rusty-pingus.db` by default. ICMP and traceroute probes open raw sockets via `surge-ping` and `socket2`, requiring `CAP_NET_RAW` (and sometimes `CAP_NET_ADMIN`).

A deployment package needs to: compile the binary, produce a small runtime image, map Docker env vars to CLI flags via an entrypoint script, expose two volume mounts (config, data), and ship an Unraid Community Applications XML template. All deployment files live under `environment/` to keep the repo root tidy.

## Goals / Non-Goals

**Goals:**
- Single `Dockerfile` at repo root; multi-stage (build → runtime).
- `environment/docker-compose.yml` as a reference stack.
- `environment/unraid/rusty-pingus.xml` for one-click Unraid CA install.
- `BIND_PORT` env var (default `8080`) drives the listen address.
- `/config` mount holds `config.toml` + `monitors.toml`; `/data` mount holds the SQLite database.
- `CAP_NET_RAW` + `CAP_NET_ADMIN` granted both in the image (via `setcap`) and declared in the compose/Unraid template.
- Run as a non-root user inside the container.

**Non-Goals:**
- Kubernetes / Helm charts.
- Automatic config.toml generation at first run (users supply their own or copy the example).
- TLS termination inside the container (delegate to a reverse proxy).
- Publishing to Docker Hub (out of scope for this change).

## Decisions

### Multi-stage Dockerfile
**Decision**: `rust:1-slim-bookworm` builder → `debian:bookworm-slim` runtime.

The builder compiles with `--release`. The runtime image only needs `ca-certificates` and `libssl3` (for outbound HTTPS monitoring via `reqwest` + `rustls-tls`). The binary is statically linked to musl or dynamically linked to glibc — glibc chosen here for simplicity since `surge-ping` and `socket2` use libc raw socket calls.

### CAP_NET_RAW: setcap + Docker cap_add
**Decision**: `setcap cap_net_raw,cap_net_admin+ep` the binary during the image build, AND declare `cap_add: [NET_RAW, NET_ADMIN]` in compose/Unraid.

`setcap` on the binary lets it acquire the capabilities even when the container runs as a non-root user (ambient capabilities). The Docker `cap_add` is required because Linux strips file capabilities if the runtime capability set doesn't include them. Using both is belt-and-suspenders and covers all Linux kernel versions.

Alternative considered: `privileged: true`. Rejected — overly broad; gives all capabilities including dangerous ones.

### Entrypoint script
**Decision**: A small `entrypoint.sh` maps env vars to CLI flags before exec'ing the binary.

The binary's CLI args are the source of truth. The entrypoint translates:
- `BIND_PORT` (default `8080`) → `--bind 0.0.0.0:${BIND_PORT}`
- Config always read from `/config/config.toml`
- Monitors always read from `/config/monitors.toml`
- DB always at `/data/rusty-pingus.db`

This keeps the binary interface stable while giving Docker users a clean env-var surface.

### directory structure
```
environment/
  docker-compose.yml
  unraid/
    rusty-pingus.xml
Dockerfile
entrypoint.sh
```

`Dockerfile` and `entrypoint.sh` at repo root (conventional Docker locations). Everything else under `environment/`.

### Unraid XML format
The Unraid CA XML uses `<Container>` with `<Config>` children for ports, paths, and env vars. Capabilities are expressed via `<ExtraParams>--cap-add=NET_RAW --cap-add=NET_ADMIN</ExtraParams>` (Unraid does not have a first-class capability field; ExtraParams passes raw `docker run` flags).

## Risks / Trade-offs

- **setcap may be stripped by some OCI runtimes**: Rootless Docker / Podman may not honour file capabilities. Mitigation: document this in comments inside the XML/compose; users on those runtimes can fall back to `privileged: true` or disable ICMP/traceroute.
- **glibc in runtime image**: Musl would be smaller but requires cross-compilation setup (`cross` crate or musl target). Not worth the CI complexity for a first packaging pass.
- **No config.toml generation**: Users must supply `/config/config.toml`. Mitigation: copy `config.toml` from the repo as the example; document in Dockerfile comments.

## Open Questions

- Should the image be published to `ghcr.io/others-git/rusty-pingus` as part of this change, or deferred to a CI release pipeline? *(Deferred — CI pipeline change handles publishing.)*
