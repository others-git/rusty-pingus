## 1. Dockerfile + entrypoint

- [ ] 1.1 Create `Dockerfile` at repo root: builder stage (`rust:1-slim-bookworm`) runs `cargo build --release`; runtime stage (`debian:bookworm-slim`) installs `ca-certificates` and `libssl3`, creates `appuser`, copies the binary. Base should be alpine.
- [ ] 1.2 In the runtime stage, run `setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/rusty-pingus` (requires `libcap2-bin` package in build layer, installed and removed after setcap).
- [ ] 1.3 Entrypoint should just be the binary call: `--bind 0.0.0.0:${BIND_PORT} --config /config/config.toml --monitors /config/monitors.toml --db /data/rusty-pingus.db`. This means that the Dockerfile will not have an entrypoint defined. Instead, it will just be a binary.
- [ ] 1.4 Set `USER appuser` in the Dockerfile.

## 2. environment/ directory

- [ ] 2.1 Create `environment/docker-compose.yml`: service with `build: .`, `cap_add: [NET_RAW, NET_ADMIN]`, `ports: ["${BIND_PORT:-8080}:${BIND_PORT:-8080}"]`, volumes for `/config` and `/data`, and `BIND_PORT` env var.
- [ ] 2.2 Create `environment/unraid/rusty-pingus.xml`: Unraid CA `<Container>` template with `<Name>`, `<Repository>` placeholder, `<Privileged>false</Privileged>`, `<ExtraParams>--cap-add=NET_RAW --cap-add=NET_ADMIN</ExtraParams>`, port config entry (default 8080), and two path config entries (`/config`, `/data`).
- [ ] 2.3 Add a `BIND_PORT` env-var `<Config>` entry to the Unraid XML with a sensible description and default of `8080`.

## 3. Verification

- [ ] 3.1 `docker build -t rusty-pingus-test .` completes without error.
- [ ] 3.2 `docker run --rm --cap-add=NET_RAW --cap-add=NET_ADMIN -v ./config.toml:/config/config.toml -v ./monitors.toml:/config/monitors.toml -v /tmp/rp-data:/data -p 8080:8080 rusty-pingus-test` starts and the UI is reachable at `http://localhost:8080`.
- [ ] 3.3 ICMP/traceroute monitors probe successfully in the running container (no raw-socket permission errors in logs).
- [ ] 3.4 Sync the `container-packaging` main spec at archive time.
