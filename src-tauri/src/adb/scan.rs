//! Finds TVs listening for legacy network debugging (`adb tcpip`, port 5555) on the local subnets.

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

/// Port Android opens for "Network debugging" / `adb tcpip 5555`.
pub const LEGACY_PORT: u16 = 5555;
/// Per-host connect timeout. LAN hosts answer in a few ms; silent addresses cost the full timeout.
const PROBE_TIMEOUT: Duration = Duration::from_millis(300);
/// Parallel probes. 254 hosts / 64 workers ≈ 4 timeouts worst case, about 1.2 s.
const MAX_WORKERS: usize = 64;
/// Larger subnets are narrowed to the /24 around this computer, so a scan stays a few seconds.
const WIDEST_PREFIX: u8 = 24;
/// /31 and /32 have no other hosts to scan.
const NARROWEST_PREFIX: u8 = 30;

/// IPv4 addresses (with prefix length) of this computer's active, non-loopback interfaces.
pub fn local_networks() -> Vec<(Ipv4Addr, u8)> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| interface.is_oper_up() && !interface.is_loopback())
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(v4) => Some((v4.ip, v4.prefixlen)),
            if_addrs::IfAddr::V6(_) => None,
        })
        .collect()
}

/// Every other host address on the given networks, each narrowed to at most a /24.
pub fn hosts_to_probe(networks: &[(Ipv4Addr, u8)]) -> Vec<Ipv4Addr> {
    let mut hosts = BTreeSet::new();
    for &(ip, prefix) in networks {
        if ip.is_loopback() || ip.is_link_local() || prefix > NARROWEST_PREFIX {
            continue;
        }
        let prefix = prefix.max(WIDEST_PREFIX);
        let mask = u32::MAX << (32 - u32::from(prefix));
        let network = u32::from(ip) & mask;
        let broadcast = network | !mask;
        hosts.extend(
            ((network + 1)..broadcast)
                .map(Ipv4Addr::from)
                .filter(|host| *host != ip),
        );
    }
    hosts.into_iter().collect()
}

/// Tries a TCP connect to `LEGACY_PORT` on every host and returns the ones that accept. Blocking.
pub fn probe(hosts: &[Ipv4Addr]) -> Vec<Ipv4Addr> {
    let next = AtomicUsize::new(0);
    let open = Mutex::new(Vec::new());
    let workers = hosts.len().min(MAX_WORKERS);
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(&host) = hosts.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let address = SocketAddr::from((host, LEGACY_PORT));
                    if TcpStream::connect_timeout(&address, PROBE_TIMEOUT).is_ok() {
                        // A poisoned lock only means another worker panicked; its data is still valid.
                        open.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push(host);
                    }
                }
            });
        }
    });
    let mut open = open
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    open.sort_unstable();
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_to_probe_should_cover_slash_24_without_self() {
        let hosts = hosts_to_probe(&[(Ipv4Addr::new(192, 168, 1, 10), 24)]);
        assert_eq!(hosts.len(), 253);
    }

    #[test]
    fn hosts_to_probe_should_exclude_network_and_broadcast() {
        let hosts = hosts_to_probe(&[(Ipv4Addr::new(192, 168, 1, 10), 24)]);
        assert_eq!(
            (hosts.first().copied(), hosts.last().copied()),
            (
                Some(Ipv4Addr::new(192, 168, 1, 1)),
                Some(Ipv4Addr::new(192, 168, 1, 254))
            )
        );
    }

    #[test]
    fn hosts_to_probe_should_narrow_wide_networks_to_slash_24() {
        let hosts = hosts_to_probe(&[(Ipv4Addr::new(10, 0, 5, 20), 16)]);
        assert_eq!(hosts.first().copied(), Some(Ipv4Addr::new(10, 0, 5, 1)));
    }

    #[test]
    fn hosts_to_probe_should_keep_small_subnets() {
        assert_eq!(
            hosts_to_probe(&[(Ipv4Addr::new(192, 168, 1, 1), 30)]),
            vec![Ipv4Addr::new(192, 168, 1, 2)]
        );
    }

    #[test]
    fn hosts_to_probe_should_skip_link_local() {
        assert_eq!(
            hosts_to_probe(&[(Ipv4Addr::new(169, 254, 3, 4), 16)]),
            Vec::<Ipv4Addr>::new()
        );
    }

    #[test]
    fn hosts_to_probe_should_merge_duplicates() {
        let network = (Ipv4Addr::new(192, 168, 1, 10), 24);
        assert_eq!(hosts_to_probe(&[network, network]).len(), 253);
    }
}
