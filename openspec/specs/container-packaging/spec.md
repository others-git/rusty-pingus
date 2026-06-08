## Purpose

Defines the container packaging for rusty-pingus: a multi-stage Dockerfile, a docker-compose reference stack, and an Unraid Community Applications XML template for one-click self-hosted deployment.

## Requirements

### Requirement: Multi-stage Dockerfile
The project SHALL provide a `Dockerfile` at the repo root that compiles rusty-pingus via a Rust builder stage and produces a minimal Alpine-based runtime image.

#### Scenario: Builder stage compiles the binary
- **WHEN** `docker build` is run in the repo root
- **THEN** the builder stage uses a `rust:alpine` image to run `cargo build --release`

#### Scenario: Runtime stage is minimal
- **WHEN** the image is built
- **THEN** the runtime stage is based on `alpine:3` and only includes `ca-certificates`, `sqlite-libs`, and the compiled binary

#### Scenario: Non-root execution
- **WHEN** the container is started
- **THEN** the binary runs as a non-root user (`appuser`) created during the image build

### Requirement: Network capability grants for raw sockets
The container image SHALL grant `CAP_NET_RAW` and `CAP_NET_ADMIN` to the binary so that ICMP ping and traceroute probes work without running the container as root.

#### Scenario: setcap applied during build
- **WHEN** the Docker image is built
- **THEN** `setcap cap_net_raw,cap_net_admin+ep` is applied to the binary in the runtime layer

#### Scenario: Compose and Unraid declare cap_add
- **WHEN** the container is started via docker-compose or the Unraid template
- **THEN** `NET_RAW` and `NET_ADMIN` are listed under `cap_add` (compose) or `ExtraParams` (Unraid XML)

### Requirement: Configurable bind port via environment variable
The container SHALL expose the HTTP bind port as a `BIND_PORT` environment variable, defaulting to `8080`.

#### Scenario: Default port
- **WHEN** `BIND_PORT` is not set
- **THEN** the server listens on `0.0.0.0:8080`

#### Scenario: Custom port
- **WHEN** `BIND_PORT=9090` is set
- **THEN** the server listens on `0.0.0.0:9090`

### Requirement: Separate config and data volume mounts
The container SHALL define two named mount points: `/config` for configuration files and `/data` for the SQLite database.

#### Scenario: Config files read from /config
- **WHEN** the container starts
- **THEN** `config.toml` is read from `/config/config.toml` and `monitors.toml` from `/config/monitors.toml`

#### Scenario: Database stored under /data
- **WHEN** the container starts
- **THEN** the SQLite database is written to `/data/rusty-pingus.db`

#### Scenario: Independent backup of config vs data
- **WHEN** a user mounts `/config` and `/data` to separate host directories
- **THEN** backing up one directory does not require backing up the other

### Requirement: Shell CMD maps env vars to CLI flags
The image SHALL use a shell-form `CMD` that expands `BIND_PORT` and passes fixed mount paths as CLI arguments to the binary at container start.

#### Scenario: Env vars translated to CLI flags
- **WHEN** the container starts
- **THEN** the binary receives `--bind 0.0.0.0:${BIND_PORT}`, `--config /config/config.toml`, `--monitors /config/monitors.toml`, and `--db /data/rusty-pingus.db`

### Requirement: docker-compose reference stack
The project SHALL provide `environment/docker-compose.yml` as a working reference configuration.

#### Scenario: Reference compose declares mounts and caps
- **WHEN** a user inspects `environment/docker-compose.yml`
- **THEN** it declares `/config` and `/data` volume mounts, `cap_add: [NET_RAW, NET_ADMIN]`, the `BIND_PORT` variable, and port mapping `${BIND_PORT}:${BIND_PORT}`

### Requirement: Unraid Community Applications XML template
The project SHALL provide `environment/unraid/rusty-pingus.xml` as a Community Applications template for Unraid.

#### Scenario: Template declares port variable
- **WHEN** a user installs via Unraid CA
- **THEN** a `<Config>` entry of `Type="Port"` with the default `8080` is shown

#### Scenario: Template declares config and data paths
- **WHEN** a user installs via Unraid CA
- **THEN** two `<Config>` entries of `Type="Path"` map host directories to `/config` and `/data`

#### Scenario: Template grants raw socket capabilities
- **WHEN** a user installs via Unraid CA
- **THEN** `<ExtraParams>` contains `--cap-add=NET_RAW --cap-add=NET_ADMIN`

### Requirement: Environment files stored under environment/
All deployment-specific files (compose, Unraid XML) SHALL live under the `environment/` directory at the repo root.

#### Scenario: environment/ directory layout
- **WHEN** a user browses the repo
- **THEN** they find `environment/docker-compose.yml` and `environment/unraid/rusty-pingus.xml`
