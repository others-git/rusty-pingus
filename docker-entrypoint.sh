#!/bin/sh
# Pattern-A startup: run as root just long enough to align ownership of the
# mounted volumes with the requested runtime user, then drop privileges and exec
# the app. This is the convention Unraid (and LinuxServer.io) images follow so
# the container can write to host-owned appdata directories.
#
# PUID/PGID default to Unraid's `nobody:users` (99:100), which owns everything
# under /mnt/user/appdata. Override them for other hosts (e.g. PUID=1000).
set -e

PUID="${PUID:-99}"
PGID="${PGID:-100}"

# Optional file-mode creation mask (Unraid convention, e.g. UMASK=022).
if [ -n "$UMASK" ]; then
    umask "$UMASK"
fi

# Repair ownership on every start: the app needs directory write access to
# create the SQLite DB (and its -wal/-shm sidecars), generate config.toml /
# monitors.toml on first run, and write logs. Re-running each boot also undoes
# host-side resets (e.g. Unraid's "Docker Safe New Permissions" reverting
# appdata to 99:100). Best-effort — warn but continue if the runtime forbids it.
for dir in /config /data; do
    mkdir -p "$dir"
    chown -R "${PUID}:${PGID}" "$dir" 2>/dev/null \
        || echo "rusty-pingus: warning: could not chown ${dir} to ${PUID}:${PGID} (continuing)"
done

# Drop root and run the app as PUID:PGID. The binary's file capabilities
# (cap_net_raw/cap_net_admin, set via setcap in the Dockerfile) are applied on
# exec regardless of the invoking user, so ICMP/traceroute probes still work.
exec su-exec "${PUID}:${PGID}" "$@"
