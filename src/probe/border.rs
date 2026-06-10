use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};
use tokio::sync::OnceCell;
use tracing::{debug, warn};

use crate::monitors::BorderMonitorConfig;
use super::icmp::{ping_once, PingError};
use super::ProbeResult;

/// Known-public destination used only for gateway discovery (never monitored).
const TRACE_TARGET: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
/// Stop the discovery walk after this many hops. Home networks reach the ISP in
/// 2–3 hops, but a virtualized egress (WSL2/Docker bridge) plus double-NAT/CGNAT
/// can add a few private hops before the first public one, so allow some headroom.
const TRACE_MAX_HOPS: u8 = 8;
/// Per-hop read timeout for the discovery walk — kept short because we are
/// probing local hops and want startup to be fast (≤ 1 s total worst case).
const WALK_PER_HOP_MS: u64 = 200;

const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_TIME_EXCEEDED: u8 = 11;
const ICMP_DEST_UNREACHABLE: u8 = 3;

/// Gateways discovered via a traceroute walk to a public endpoint.
/// Cached for the process lifetime — topology rarely changes while running.
#[derive(Debug)]
struct DetectedGateways {
    /// Last RFC-1918/CGNAT hop before traffic exits to the public internet — the
    /// edge gateway leaving the LAN. Displayed as "Local GW".
    local_gw: String,
    /// First public-IP hop (ISP's edge router) — the ISP gateway. `None` when the
    /// walk exited private space but no public hop responded within `TRACE_MAX_HOPS`.
    /// Displayed as "Border GW".
    isp_gw: Option<String>,
}

static DETECTED_GWS: OnceCell<Option<DetectedGateways>> = OnceCell::const_new();

/// Localize connectivity faults by probing both the local (egress) gateway and
/// the ISP gateway, then an upstream reference:
///
/// - all up → `ok`
/// - local gw down → `lan_down`
/// - local gw up, ISP gw down → `isp_gw_down`
/// - local gw up, ISP gw up (or not detected), upstream down → `isp_down`
///
/// Gateways are auto-detected via a traceroute walk when not configured
/// explicitly, and the result is cached for the process lifetime.
pub async fn run(cfg: &BorderMonitorConfig) -> ProbeResult {
    let timeout = Duration::from_millis(cfg.timeout_ms);
    let (local_gw, isp_gw) = resolve_gateways(cfg).await;

    let local_gw = match local_gw {
        Some(g) => g,
        None => {
            let endpoint = format!("auto → {}", cfg.upstream);
            return ProbeResult::down(&cfg.name, "border", &endpoint, "gateway_not_detected")
                .with_detail(
                    "local gateway could not be detected — check network \
                     or configure gateway explicitly"
                    .to_string(),
                );
        }
    };

    // Path: local_gw (LAN edge, last private hop) → isp_gw (first public hop) → upstream.
    let endpoint = match &isp_gw {
        Some(isp) => format!("{local_gw} → {isp} → {}", cfg.upstream),
        None => format!("{local_gw} → {}", cfg.upstream),
    };

    // Probe all targets in parallel so a non-responding gateway (very common —
    // most home routers block ICMP echo) does not delay the upstream check.
    let (gw_result, isp_result, up_result) = if let Some(ref isp) = isp_gw {
        let (gw, isp_r, up) = tokio::join!(
            ping_once(&local_gw, timeout),
            ping_once(isp, timeout),
            ping_once(&cfg.upstream, timeout),
        );
        (gw, Some(isp_r), up)
    } else {
        let (gw, up) = tokio::join!(
            ping_once(&local_gw, timeout),
            ping_once(&cfg.upstream, timeout),
        );
        (gw, None, up)
    };

    // A raw-socket privilege failure is not a connectivity fault — surface it clearly.
    let privilege_err = matches!(gw_result, Err(PingError::Privilege))
        || isp_result
            .as_ref()
            .is_some_and(|r| matches!(r, Err(PingError::Privilege)))
        || matches!(up_result, Err(PingError::Privilege));
    if privilege_err {
        warn!(monitor = %cfg.name, "Border probe socket error — check CAP_NET_RAW privileges");
        return ProbeResult::down(&cfg.name, "border", &endpoint, "privilege_error");
    }

    let gw_rtt = gw_result.as_ref().ok().copied();
    let isp_rtt = isp_result.as_ref().and_then(|r| r.as_ref().ok().copied());
    let up_rtt = up_result.as_ref().ok().copied();

    debug!(
        monitor = %cfg.name,
        local_gw = %local_gw,
        isp_gw = ?isp_gw,
        upstream = %cfg.upstream,
        gw_ok = gw_result.is_ok(),
        isp_gw_ok = ?isp_result.as_ref().map(|r| r.is_ok()),
        upstream_ok = up_result.is_ok(),
        "Border probe"
    );

    classify_border(
        &cfg.name,
        &endpoint,
        gw_result.is_ok(),
        isp_result.map(|r| r.is_ok()),
        up_result.is_ok(),
        gw_rtt,
        isp_rtt,
        up_rtt,
    )
}

/// Resolve both gateways: prefer explicit config, fall back to auto-detection.
///
/// Returns `(local_gw, isp_gw)` where:
/// - `local_gw` — last private-IP hop (the LAN's edge gateway); shown as "Local GW".
///   The traceroute's last-private hop is preferred; the routing-table default
///   gateway is only a fallback when the walk yields nothing (e.g. no privilege).
/// - `isp_gw` — first public-IP hop (the ISP gateway); shown as "Border GW". `None`
///   when no public hop responded.
///
/// When `gateway` is explicitly set the traceroute walk is skipped entirely.
async fn resolve_gateways(cfg: &BorderMonitorConfig) -> (Option<String>, Option<String>) {
    // Explicit local gateway — trust the user, skip detection.
    if let Some(ref gw) = cfg.gateway {
        return (Some(gw.clone()), cfg.isp_gateway.clone());
    }

    // Auto-detect via traceroute walk, cached for the process lifetime.
    // Uses WALK_PER_HOP_MS (not the probe timeout) so detection is fast.
    let detected = DETECTED_GWS
        .get_or_init(|| async {
            tokio::task::spawn_blocking(|| {
                walk_for_gateways(Duration::from_millis(WALK_PER_HOP_MS))
            })
            .await
            .ok()
            .flatten()
        })
        .await;

    // Local GW = the traceroute's last private hop (the actual LAN edge). Fall back
    // to the routing-table default gateway only when the walk found nothing.
    let local_gw = detected
        .as_ref()
        .map(|d| d.local_gw.clone())
        .or_else(detect_default_gateway);

    // Border GW = first public hop (ISP gateway), or an explicit override.
    let isp_gw = cfg
        .isp_gateway
        .clone()
        .or_else(|| detected.as_ref().and_then(|d| d.isp_gw.clone()));

    (local_gw, isp_gw)
}

/// Pure classification of a border cycle — network-free, for testing.
#[allow(clippy::too_many_arguments)]
fn classify_border(
    name: &str,
    endpoint: &str,
    local_gw_ok: bool,
    isp_gw_ok: Option<bool>,
    upstream_ok: bool,
    gw_rtt: Option<u64>,
    isp_rtt: Option<u64>,
    up_rtt: Option<u64>,
) -> ProbeResult {
    let fmt = |rtt: Option<u64>| rtt.map(|r| format!("{r}ms")).unwrap_or_else(|| "—".to_string());
    let rtts = if isp_gw_ok.is_some() {
        format!(
            "local {}, isp {}, upstream {}",
            fmt(gw_rtt),
            fmt(isp_rtt),
            fmt(up_rtt)
        )
    } else {
        format!("local {}, upstream {}", fmt(gw_rtt), fmt(up_rtt))
    };

    // If upstream is reachable the full path is working. Gateways very commonly
    // block ICMP echo from LAN hosts (firewall rule) while still forwarding
    // traffic — checking outside-in prevents false LAN_DOWN alerts.
    if upstream_ok {
        return ProbeResult::up(name, "border", endpoint, up_rtt.unwrap_or(0))
            .with_detail(format!("ok ({rtts})"));
    }

    // Upstream is down — localise the failure from ISP edge inward.
    if isp_gw_ok == Some(true) {
        // ISP edge responds but upstream does not → internet is down.
        return ProbeResult::down(name, "border", endpoint, "isp_down")
            .with_detail(format!("isp_down — upstream unreachable ({rtts})"));
    }
    if isp_gw_ok == Some(false) && local_gw_ok {
        // ISP edge unreachable but local gateway is up → ISP gateway is down.
        return ProbeResult::down(name, "border", endpoint, "isp_gw_down")
            .with_detail(format!("isp_gw_down — ISP gateway unreachable ({rtts})"));
    }
    if local_gw_ok {
        // Local gateway responds but nothing beyond it does → internet is down.
        return ProbeResult::down(name, "border", endpoint, "isp_down")
            .with_detail(format!("isp_down — upstream unreachable ({rtts})"));
    }

    // Nothing is reachable — the local network itself is down.
    ProbeResult::down(name, "border", endpoint, "lan_down")
        .with_detail(format!("lan_down — local gateway unreachable ({rtts})"))
}

/// Blocking: walk hops toward `TRACE_TARGET` (8.8.8.8), returning the last
/// private-IP hop (local/egress gateway) and the first public-IP hop (ISP
/// gateway). Returns `None` when no hop can be probed or no private-IP hop is
/// seen (host already on a public address).
///
/// Prefers an **unprivileged DGRAM ICMP socket** on Linux — the same socket
/// family the echo probes use — so detection works wherever `ping` does, without
/// `CAP_NET_RAW`. Falls back to a RAW socket (needs privilege) on other platforms
/// or if the DGRAM walk yields nothing.
fn walk_for_gateways(per_hop: Duration) -> Option<DetectedGateways> {
    #[cfg(target_os = "linux")]
    if let Some(gws) = walk_dgram_linux(per_hop) {
        return Some(gws);
    }
    walk_raw(per_hop)
}

/// Shared hop-selection loop: probe increasing TTLs, recording the last private
/// hop and stopping at the first public one. `probe(ttl)` returns the responding
/// router's address for that TTL (or `None` if the hop didn't respond in time).
fn select_gateways<F>(mut probe: F) -> Option<DetectedGateways>
where
    F: FnMut(u8) -> Option<Ipv4Addr>,
{
    let mut last_private: Option<Ipv4Addr> = None;
    let mut first_public: Option<Ipv4Addr> = None;

    for ttl in 1..=TRACE_MAX_HOPS {
        if let Some(ip) = probe(ttl) {
            if is_private_ipv4(ip) {
                last_private = Some(ip);
            } else if first_public.is_none() {
                first_public = Some(ip);
                break; // found both; exit early
            }
        }
        // No response at this TTL — continue (a router may drop ICMP Time-Exceeded
        // while the next hop still answers).
    }

    Some(DetectedGateways {
        local_gw: last_private?.to_string(),
        isp_gw: first_public.map(|ip| ip.to_string()),
    })
}

/// RAW-socket walk: requires `CAP_NET_RAW`. Sends ICMP echoes with increasing TTL
/// and reads the Time-Exceeded / Echo-Reply directly off a raw ICMP socket.
fn walk_raw(per_hop: Duration) -> Option<DetectedGateways> {
    let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).ok()?;
    let _ = socket.set_read_timeout(Some(per_hop));

    let dest: socket2::SockAddr = SocketAddr::V4(SocketAddrV4::new(TRACE_TARGET, 0)).into();
    let ident = trace_ident();

    select_gateways(|ttl| {
        let _ = socket.set_ttl(ttl as u32);
        let seq = ttl as u16;
        let packet = build_echo(ident, seq);
        let start = Instant::now();
        if socket.send_to(&packet, &dest).is_err() {
            return None;
        }
        let deadline = start + per_hop;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let _ = socket.set_read_timeout(Some(deadline - now));
            let mut buf = [MaybeUninit::<u8>::uninit(); 1500];
            match socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    // SAFETY: recv_from filled exactly `n` bytes.
                    let data = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
                    if let Some((_, rid, rseq)) = parse_icmp(data) {
                        if rid == ident && rseq == seq {
                            return from.as_socket_ipv4().map(|s| *s.ip());
                        }
                        // Not our packet — keep waiting until the deadline.
                    }
                }
                Err(_) => return None, // timeout or error; this hop did not respond
            }
        }
    })
}

/// Unprivileged DGRAM-ICMP walk (Linux only). A ping socket can't receive the
/// intermediate Time-Exceeded messages via `recv()`, but enabling `IP_RECVERR`
/// routes those ICMP errors to the socket's *error queue*; `recvmsg(MSG_ERRQUEUE)`
/// then yields the offending router's address (`SO_EE_OFFENDER`). This needs no
/// `CAP_NET_RAW`, so it works in containers/WSL the same way `ping` does.
#[cfg(target_os = "linux")]
fn walk_dgram_linux(per_hop: Duration) -> Option<DetectedGateways> {
    use std::os::fd::AsRawFd;

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4)).ok()?;
    let fd = socket.as_raw_fd();

    // Deliver ICMP errors to the error queue so we can read intermediate hops.
    let on: libc::c_int = 1;
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_RECVERR,
            &on as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return None;
    }

    let dest: socket2::SockAddr = SocketAddr::V4(SocketAddrV4::new(TRACE_TARGET, 0)).into();
    let ident = trace_ident();
    let per_hop_ms = per_hop.as_millis().min(i32::MAX as u128) as libc::c_int;

    select_gateways(|ttl| {
        let _ = socket.set_ttl(ttl as u32);
        let packet = build_echo(ident, ttl as u16);
        if socket.send_to(&packet, &dest).is_err() {
            return None;
        }
        // Block (up to the per-hop budget) for an error-queue event, then read the
        // offender. `POLLERR` is reported in `revents` regardless of `events`.
        let mut pfd = libc::pollfd { fd, events: 0, revents: 0 };
        let pr = unsafe { libc::poll(&mut pfd, 1, per_hop_ms) };
        if pr <= 0 {
            return None; // no response within the budget
        }
        recv_errqueue_offender(fd)
    })
}

/// Read one ICMP error off the socket error queue and return the offending
/// router's IPv4 address (the `SO_EE_OFFENDER` sockaddr that trails the
/// `sock_extended_err` in the `IP_RECVERR` control message).
#[cfg(target_os = "linux")]
fn recv_errqueue_offender(fd: libc::c_int) -> Option<Ipv4Addr> {
    // SAFETY: all pointers below reference stack buffers that outlive the
    // `recvmsg` call, and the cmsg fields are only read after the kernel reports
    // it populated them.
    unsafe {
        let mut data = [0u8; 512];
        let mut ctrl = [0u8; 512];
        let mut iov = libc::iovec {
            iov_base: data.as_mut_ptr() as *mut libc::c_void,
            iov_len: data.len(),
        };
        let mut from: libc::sockaddr_storage = std::mem::zeroed();
        let mut msg: libc::msghdr = std::mem::zeroed();
        msg.msg_name = &mut from as *mut _ as *mut libc::c_void;
        msg.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = ctrl.as_mut_ptr() as *mut libc::c_void;
        msg.msg_controllen = ctrl.len() as _;

        if libc::recvmsg(fd, &mut msg, libc::MSG_ERRQUEUE) < 0 {
            return None;
        }

        let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
        while !cmsg.is_null() {
            let c = &*cmsg;
            if c.cmsg_level == libc::IPPROTO_IP && c.cmsg_type == libc::IP_RECVERR {
                let ee = libc::CMSG_DATA(cmsg) as *const libc::sock_extended_err;
                // SO_EE_OFFENDER(ee): sockaddr immediately following the struct.
                let offender = (ee as *const u8)
                    .add(std::mem::size_of::<libc::sock_extended_err>())
                    as *const libc::sockaddr;
                if (*offender).sa_family == libc::AF_INET as libc::sa_family_t {
                    let sin = offender as *const libc::sockaddr_in;
                    return Some(Ipv4Addr::from(u32::from_be((*sin).sin_addr.s_addr)));
                }
            }
            cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
        }
        None
    }
}

/// Returns `true` for RFC 1918 private, RFC 3927 link-local, and RFC 6598
/// CGNAT (100.64.0.0/10) addresses. The walk must treat CGNAT hops as private
/// so it keeps walking past the ISP's shared-address layer to find the first
/// truly public IP.
fn is_private_ipv4(addr: Ipv4Addr) -> bool {
    let [a, b, ..] = addr.octets();
    a == 10
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
        || (a == 100 && (64..=127).contains(&b)) // RFC 6598 CGNAT
}

/// Build an 8-byte ICMP Echo Request header + 8-byte zero payload, with checksum.
fn build_echo(id: u16, seq: u16) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = ICMP_ECHO_REQUEST;
    // p[1] code = 0; p[2..4] checksum filled below
    p[4..6].copy_from_slice(&id.to_be_bytes());
    p[6..8].copy_from_slice(&seq.to_be_bytes());
    let cks = checksum(&p);
    p[2..4].copy_from_slice(&cks.to_be_bytes());
    p
}

fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Parse a raw IPv4 packet, returning `(icmp_type, id, seq)` for an echo we sent.
/// Echo-Reply: id/seq in the outer ICMP header.
/// Time-Exceeded / Dest-Unreachable: id/seq in the embedded original datagram.
fn parse_icmp(buf: &[u8]) -> Option<(u8, u16, u16)> {
    if buf.len() < 20 {
        return None;
    }
    let ihl = (buf[0] & 0x0f) as usize * 4;
    if buf.len() < ihl + 8 {
        return None;
    }
    let icmp = &buf[ihl..];
    let typ = icmp[0];
    match typ {
        ICMP_ECHO_REPLY => {
            let id = u16::from_be_bytes([icmp[4], icmp[5]]);
            let seq = u16::from_be_bytes([icmp[6], icmp[7]]);
            Some((typ, id, seq))
        }
        ICMP_TIME_EXCEEDED | ICMP_DEST_UNREACHABLE => {
            let inner = &icmp[8..];
            if inner.len() < 28 {
                return None; // 20 inner IP + 8 inner ICMP minimum
            }
            let inner_ihl = (inner[0] & 0x0f) as usize * 4;
            if inner.len() < inner_ihl + 8 {
                return None;
            }
            let inner_icmp = &inner[inner_ihl..];
            let id = u16::from_be_bytes([inner_icmp[4], inner_icmp[5]]);
            let seq = u16::from_be_bytes([inner_icmp[6], inner_icmp[7]]);
            Some((typ, id, seq))
        }
        _ => None,
    }
}

fn trace_ident() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    ((nanos ^ std::process::id()) & 0xffff) as u16
}

/// Best-effort routing-table fallback when the traceroute walk yields nothing.
fn detect_default_gateway() -> Option<String> {
    #[cfg(target_os = "linux")]
    return detect_gateway_linux();
    #[cfg(target_os = "macos")]
    return detect_gateway_macos();
    #[cfg(target_os = "windows")]
    return detect_gateway_windows();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    None
}

#[cfg(target_os = "linux")]
fn detect_gateway_linux() -> Option<String> {
    let content = std::fs::read_to_string("/proc/net/route").ok()?;
    for line in content.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let _iface = fields.next()?;
        let dest = fields.next()?;
        let gateway = fields.next()?;
        if dest == "00000000" {
            let raw = u32::from_str_radix(gateway, 16).ok()?;
            let o = raw.to_le_bytes();
            return Some(format!("{}.{}.{}.{}", o[0], o[1], o[2], o[3]));
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn detect_gateway_macos() -> Option<String> {
    let out = std::process::Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .ok()?;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(rest) = line.trim().strip_prefix("gateway:") {
            let gw = rest.trim();
            if !gw.is_empty() {
                return Some(gw.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn detect_gateway_windows() -> Option<String> {
    let out = std::process::Command::new("route")
        .args(["print", "0.0.0.0"])
        .output()
        .ok()?;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 3
            && cols[0] == "0.0.0.0"
            && cols[1] == "0.0.0.0"
            && cols[2].parse::<std::net::IpAddr>().is_ok()
        {
            return Some(cols[2].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{classify_border, is_private_ipv4};
    use std::net::Ipv4Addr;

    // ── classify_border ───────────────────────────────────────────────────────

    #[test]
    fn all_reachable_is_up_ok() {
        let r = classify_border("b", "gw → isp → up", true, Some(true), true, Some(2), Some(8), Some(15));
        assert_eq!(r.status, "up");
        assert_eq!(r.response_time_ms, Some(15));
        assert!(r.detail.as_deref().unwrap().starts_with("ok "));
    }

    #[test]
    fn all_reachable_without_isp_gw_is_up_ok() {
        let r = classify_border("b", "gw → up", true, None, true, Some(2), None, Some(15));
        assert_eq!(r.status, "up");
        assert_eq!(r.response_time_ms, Some(15));
        assert!(r.detail.as_deref().unwrap().starts_with("ok "));
    }

    #[test]
    fn isp_gw_down_is_isp_gw_down() {
        let r = classify_border("b", "gw → isp → up", true, Some(false), false, Some(2), None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("isp_gw_down"));
        assert!(r.detail.as_deref().unwrap().contains("isp_gw_down"));
    }

    #[test]
    fn isp_gw_up_upstream_down_is_isp_down() {
        let r = classify_border("b", "gw → isp → up", true, Some(true), false, Some(2), Some(8), None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("isp_down"));
        assert!(r.detail.as_deref().unwrap().contains("isp_down"));
    }

    #[test]
    fn gateway_up_upstream_down_no_isp_gw_is_isp_down() {
        let r = classify_border("b", "gw → up", true, None, false, Some(2), None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("isp_down"));
    }

    #[test]
    fn gateway_down_is_lan_down() {
        let r = classify_border("b", "gw → up", false, None, false, None, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("lan_down"));
        assert!(r.detail.as_deref().unwrap().contains("lan_down"));
    }

    #[test]
    fn gateway_icmp_blocked_but_upstream_ok_is_ok() {
        // Routers commonly block ICMP echo from LAN hosts but still forward traffic.
        // If the upstream is reachable the path is working — not a LAN failure.
        let r = classify_border("b", "gw → up", false, None, true, None, None, Some(15));
        assert_eq!(r.status, "up");
        assert_eq!(r.failure_reason, None);
        assert!(r.detail.as_deref().unwrap().starts_with("ok "));
    }

    #[test]
    fn all_unreachable_is_lan_down() {
        let r = classify_border("b", "gw → up", false, None, false, None, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("lan_down"));
    }

    #[test]
    fn isp_gw_and_local_gw_both_down_is_lan_down() {
        // Both GWs unreachable + upstream unreachable → actual LAN failure.
        let r = classify_border("b", "gw → isp → up", false, Some(false), false, None, None, None);
        assert_eq!(r.status, "down");
        assert_eq!(r.failure_reason.as_deref(), Some("lan_down"));
    }

    // ── is_private_ipv4 ───────────────────────────────────────────────────────

    #[test]
    fn private_ranges_detected() {
        assert!(is_private_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(169, 254, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(100, 64, 0, 1)));   // CGNAT
        assert!(is_private_ipv4(Ipv4Addr::new(100, 127, 255, 255))); // CGNAT edge
    }

    #[test]
    fn public_addresses_not_private() {
        assert!(!is_private_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 15, 0, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 32, 0, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(100, 63, 255, 255))); // just outside CGNAT
        assert!(!is_private_ipv4(Ipv4Addr::new(100, 128, 0, 0)));    // just outside CGNAT
    }
}
