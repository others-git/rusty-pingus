## Why

rusty-pingus has no container packaging, making it difficult to deploy on self-hosted platforms like Unraid. Adding a Dockerfile, Unraid Community Applications XML template, and a structured `environment/` directory gives home-lab users a one-click install path with proper capability grants for ICMP and traceroute probes.

## What Changes

- New `Dockerfile` at the repo root: multi-stage Rust build → minimal runtime image, runs as a non-root user with `CAP_NET_RAW` and `CAP_NET_ADMIN` retained.
- New `environment/` directory containing all deployment-related files:
  - `docker-compose.yml` — reference compose stack.
  - `unraid/rusty-pingus.xml` — Community Applications template with volume mounts, env-var port binding, and capability declarations.
- Bind port, data directory, and config directory are all configurable via environment variables / Docker volume mounts.
- Data (`/data`) and config (`/config`) directories are separate mounts so users can back them up independently.

## Capabilities

### New Capabilities

- `container-packaging`: Dockerfile, compose reference, and Unraid CA XML for deploying rusty-pingus as a container with correct network capabilities.

### Modified Capabilities

## Impact

- New files only — no changes to existing Rust source, frontend assets, or existing spec.
- Requires `CAP_NET_RAW` (ICMP, traceroute raw sockets) and `CAP_NET_ADMIN` (some traceroute implementations); Unraid template grants both.
- Default bind port: 8080 (overridable via `BIND_PORT` env var).
- SQLite database lives under the `/data` mount; `monitors.toml` (config) lives under `/config`.
