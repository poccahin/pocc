/// L0 Kinetic Trust Root — Edge Device Identity & Tunnel Daemon
///
/// This crate runs on the physical edge robot node.
/// It manages:
/// - Secure device identity bootstrap (`did_genesis`)
/// - Inbound P2P tunnel handling with concurrency limits (`tunnel_daemon`)
/// - Inbound HTTP intent firewall (`aegis_interceptor`)

pub mod tunnel_daemon;
