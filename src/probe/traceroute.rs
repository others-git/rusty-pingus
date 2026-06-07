use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};
use tracing::{debug, warn};

use crate::db::{TraceHopInput, TraceRunInput};
use crate::monitors::TracerouteMonitorConfig;
use super::ProbeResult;

const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_TIME_EXCEEDED: u8 = 11;
const ICMP_DEST_UNREACHABLE: u8 = 3;

/// Outcome of a blocking traceroute walk: the per-hop stats plus whether the
/// destination replied. `error` is set for clean failures (privilege/socket).
struct TraceOutcome {
    hops: Vec<TraceHopInput>,
    reached: bool,
    dest_addr: Option<Ipv4Addr>,
    dest_rtt_us: Option<u64>,
    error: Option<&'static str>,
}

/// Run a traceroute probe: resolve the target, walk the path with increasing TTL,
/// and produce both a dashboard summary `ProbeResult` and the per-run hop data to
/// persist (`None` when the run produced no storable path, e.g. on a hard error).
pub async fn run(cfg: &TracerouteMonitorConfig) -> (ProbeResult, Option<TraceRunInput>) {
    let endpoint = cfg.host.clone();

    // Resolve the target (sync parse first, then DNS) on the async side.
    let ip: IpAddr = match cfg.host.parse() {
        Ok(ip) => ip,
        Err(_) => match tokio::net::lookup_host(format!("{}:0", cfg.host)).await {
            Ok(mut addrs) => match addrs.next() {
                Some(a) => a.ip(),
                None => return (ProbeResult::down(&cfg.name, "traceroute", &endpoint, "dns_error"), None),
            },
            Err(_) => return (ProbeResult::down(&cfg.name, "traceroute", &endpoint, "dns_error"), None),
        },
    };

    let dest = match ip {
        IpAddr::V4(v4) => v4,
        // IPv6 traceroute (ICMPv6 hop-limit + Time-Exceeded) is out of scope for v1.
        IpAddr::V6(_) => {
            return (
                ProbeResult::down(&cfg.name, "traceroute", &endpoint, "ipv6_unsupported"),
                None,
            )
        }
    };

    let max_hops = cfg.max_hops.clamp(1, 64) as u8;
    let queries = cfg.queries_per_hop.max(1);
    // Per-query reply wait, capped so an all-timeout trace can't run away.
    let per_query = Duration::from_millis(cfg.timeout_ms.clamp(100, 2000));
    let ident = rand_ident();

    // Raw sockets block; run the walk on a blocking thread.
    let outcome = tokio::task::spawn_blocking(move || trace_ipv4(dest, max_hops, queries, per_query, ident))
        .await
        .unwrap_or(TraceOutcome { hops: vec![], reached: false, dest_addr: None, dest_rtt_us: None, error: Some("join_error") });

    // Hard errors → a clean down result, no stored run.
    if let Some(err) = outcome.error {
        if err == "privilege" {
            warn!(monitor = %cfg.name, "Traceroute raw socket error — check CAP_NET_RAW privileges");
            return (ProbeResult::down(&cfg.name, "traceroute", &endpoint, "privilege_error"), None);
        }
        return (ProbeResult::down(&cfg.name, "traceroute", &endpoint, err), None);
    }

    let hop_count = outcome.hops.len();
    let responded = outcome.hops.iter().any(|h| h.addr.is_some());
    let run = TraceRunInput { reached: outcome.reached, hops: outcome.hops };

    let summary = if outcome.reached {
        let dest_ms = outcome.dest_rtt_us.map(|us| us / 1000).unwrap_or(0);
        let detail = match outcome.dest_addr {
            Some(d) => format!("{hop_count} hops · reached {d}"),
            None => format!("{hop_count} hops · reached"),
        };
        debug!(monitor = %cfg.name, host = %cfg.host, hop_count, "Traceroute reached destination");
        ProbeResult::up(&cfg.name, "traceroute", &endpoint, dest_ms).with_detail(detail)
    } else if responded {
        // Path partially visible but destination didn't reply within max hops.
        debug!(monitor = %cfg.name, host = %cfg.host, hop_count, "Traceroute did not reach destination");
        ProbeResult::up(&cfg.name, "traceroute", &endpoint, 0)
            .with_detail(format!("{hop_count} hops · dest not reached"))
    } else {
        ProbeResult::down(&cfg.name, "traceroute", &endpoint, "no_reply")
    };

    (summary, Some(run))
}

/// Blocking IPv4 ICMP traceroute: for each TTL, send `queries` echoes and collect
/// the responding hop address and RTTs, stopping when the destination replies.
fn trace_ipv4(
    dest: Ipv4Addr,
    max_hops: u8,
    queries: u32,
    per_query: Duration,
    ident: u16,
) -> TraceOutcome {
    let socket = match Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)) {
        Ok(s) => s,
        Err(_) => {
            return TraceOutcome { hops: vec![], reached: false, dest_addr: None, dest_rtt_us: None, error: Some("privilege") }
        }
    };
    let _ = socket.set_read_timeout(Some(per_query));

    let dest_sa: SocketAddr = SocketAddr::V4(SocketAddrV4::new(dest, 0));
    let dest_sock = dest_sa.into();

    let mut hops: Vec<TraceHopInput> = Vec::new();
    let mut reached = false;
    let mut dest_addr: Option<Ipv4Addr> = None;
    let mut dest_rtt_us: Option<u64> = None;

    for ttl in 1..=max_hops {
        if socket.set_ttl(ttl as u32).is_err() {
            return TraceOutcome { hops, reached, dest_addr, dest_rtt_us, error: Some("socket_error") };
        }

        let mut hop_addr: Option<Ipv4Addr> = None;
        let mut rtts_us: Vec<u64> = Vec::new();
        let mut hop_reached = false;

        for q in 0..queries {
            // Encode (ttl, query) into the sequence so replies can be matched.
            let seq = ((ttl as u16) << 8) | (q as u16 & 0x00ff);
            let packet = build_echo(ident, seq);
            let start = Instant::now();
            if socket.send_to(&packet, &dest_sock).is_err() {
                continue;
            }

            let deadline = start + per_query;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let _ = socket.set_read_timeout(Some(deadline - now));
                let mut buf = [MaybeUninit::<u8>::uninit(); 1500];
                match socket.recv_from(&mut buf) {
                    Ok((n, from)) => {
                        // SAFETY: the kernel filled `n` bytes of `buf`.
                        let data = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
                        if let Some((typ, rid, rseq)) = parse_icmp_reply(data) {
                            if rid == ident && rseq == seq {
                                let src = from.as_socket_ipv4().map(|s| *s.ip());
                                hop_addr = src.or(hop_addr);
                                let rtt = start.elapsed().as_micros() as u64;
                                rtts_us.push(rtt);
                                if typ == ICMP_ECHO_REPLY || typ == ICMP_DEST_UNREACHABLE {
                                    hop_reached = typ == ICMP_ECHO_REPLY;
                                    if hop_reached {
                                        dest_addr = src.or(dest_addr);
                                        if dest_rtt_us.is_none() {
                                            dest_rtt_us = Some(rtt);
                                        }
                                    }
                                }
                                break;
                            }
                            // Not our packet — keep waiting until the deadline.
                        }
                    }
                    // Timeout / would-block: this query got no reply.
                    Err(_) => break,
                }
            }
        }

        let received = rtts_us.len() as u32;
        let (min_us, avg_us, max_us) = stats(&rtts_us);
        hops.push(TraceHopInput {
            hop_no: ttl as i64,
            addr: hop_addr.map(|a| a.to_string()),
            min_us,
            avg_us,
            max_us,
            loss: (queries - received) as i64,
        });

        if hop_reached {
            reached = true;
            break;
        }
    }

    TraceOutcome { hops, reached, dest_addr, dest_rtt_us, error: None }
}

/// min/avg/max of the responding RTTs (microseconds), or all-None if empty.
fn stats(rtts: &[u64]) -> (Option<i64>, Option<i64>, Option<i64>) {
    if rtts.is_empty() {
        return (None, None, None);
    }
    let min = *rtts.iter().min().unwrap() as i64;
    let max = *rtts.iter().max().unwrap() as i64;
    let avg = (rtts.iter().sum::<u64>() / rtts.len() as u64) as i64;
    (Some(min), Some(avg), Some(max))
}

/// Build an 8-byte ICMP Echo Request header + 8-byte payload, with checksum.
fn build_echo(id: u16, seq: u16) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = ICMP_ECHO_REQUEST;
    p[1] = 0; // code
    // p[2..4] checksum, filled below
    p[4..6].copy_from_slice(&id.to_be_bytes());
    p[6..8].copy_from_slice(&seq.to_be_bytes());
    // p[8..16] payload left as zeros
    let cks = checksum(&p);
    p[2..4].copy_from_slice(&cks.to_be_bytes());
    p
}

/// Standard internet checksum (one's-complement 16-bit sum) over `data`.
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

/// Parse a received raw-IPv4 ICMP packet, returning `(icmp_type, id, seq)` for the
/// echo we sent. For Echo-Reply the id/seq are in the outer ICMP header; for
/// Time-Exceeded / Dest-Unreachable they are in the echoed original datagram.
fn parse_icmp_reply(buf: &[u8]) -> Option<(u8, u16, u16)> {
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
            // After the 8-byte ICMP header comes the original IP header + datagram.
            let inner = &icmp[8..];
            if inner.len() < 20 {
                return None;
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

fn rand_ident() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    ((nanos ^ (std::process::id())) & 0xffff) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_of_known_echo_is_stable() {
        // Two builds of the same id/seq produce an identical, non-zero checksum.
        let a = build_echo(0x1234, 0x0102);
        let b = build_echo(0x1234, 0x0102);
        assert_eq!(a, b);
        assert_ne!(&a[2..4], &[0, 0]);
    }

    #[test]
    fn stats_empty_is_all_none() {
        assert_eq!(stats(&[]), (None, None, None));
    }

    #[test]
    fn stats_computes_min_avg_max() {
        assert_eq!(stats(&[10, 20, 30]), (Some(10), Some(20), Some(30)));
    }

    #[test]
    fn parse_echo_reply_extracts_id_seq() {
        // Minimal IPv4 header (ihl=5) + ICMP echo reply with id=0xABCD seq=0x0007.
        let mut pkt = vec![0u8; 20 + 8];
        pkt[0] = 0x45; // version 4, ihl 5
        let icmp = 20;
        pkt[icmp] = ICMP_ECHO_REPLY;
        pkt[icmp + 4..icmp + 6].copy_from_slice(&0xABCDu16.to_be_bytes());
        pkt[icmp + 6..icmp + 8].copy_from_slice(&0x0007u16.to_be_bytes());
        assert_eq!(parse_icmp_reply(&pkt), Some((ICMP_ECHO_REPLY, 0xABCD, 0x0007)));
    }

    #[test]
    fn parse_time_exceeded_extracts_inner_id_seq() {
        // Outer IP+ICMP(11) then 4 unused + inner IP(ihl=5) + inner ICMP echo.
        let mut pkt = vec![0u8; 20 + 8 + 20 + 8];
        pkt[0] = 0x45;
        let icmp = 20;
        pkt[icmp] = ICMP_TIME_EXCEEDED;
        let inner = icmp + 8;
        pkt[inner] = 0x45; // inner IP ihl 5
        let inner_icmp = inner + 20;
        pkt[inner_icmp] = ICMP_ECHO_REQUEST;
        pkt[inner_icmp + 4..inner_icmp + 6].copy_from_slice(&0x1111u16.to_be_bytes());
        pkt[inner_icmp + 6..inner_icmp + 8].copy_from_slice(&0x2222u16.to_be_bytes());
        assert_eq!(parse_icmp_reply(&pkt), Some((ICMP_TIME_EXCEEDED, 0x1111, 0x2222)));
    }
}
