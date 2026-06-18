use super::PositionBroadcast;
use socket2::{Socket, Domain, Type, Protocol, SockAddr};
use std::net::{SocketAddrV4, Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_MULTICAST_GROUP: &str = "239.255.0.1";
pub const DEFAULT_MULTICAST_PORT: u16 = 6789;
pub const BROADCAST_INTERVAL_MS: u64 = 40;

pub struct MulticastBroadcaster {
    socket: Option<UdpSocket>,
    multicast_addr: SocketAddrV4,
    session_id: u64,
    sequence: Arc<AtomicU64>,
    enabled: bool,
    local_addr: Option<SocketAddrV4>,
}

impl MulticastBroadcaster {
    pub fn new(multicast_ip: &str, port: u16) -> Result<Self, String> {
        let group_ip: Ipv4Addr = multicast_ip.parse().map_err(|e| format!("Invalid multicast IP: {}", e))?;
        let multicast_addr = SocketAddrV4::new(group_ip, port);
        Ok(Self {
            socket: None,
            multicast_addr,
            session_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            sequence: Arc::new(AtomicU64::new(0)),
            enabled: false,
            local_addr: None,
        })
    }

    pub fn start(&mut self, interface_ip: Option<&str>) -> Result<(), String> {
        let domain = Domain::IPV4;
        let sock_type = Type::DGRAM;
        let proto = Protocol::UDP;
        let socket = Socket::new(domain, sock_type, Some(proto))
            .map_err(|e| format!("Failed to create UDP socket: {}", e))?;

        socket.set_reuse_address(true)
            .map_err(|e| format!("Failed to set reuse address: {}", e))?;

        let bind_addr: SocketAddrV4 = if let Some(ip) = interface_ip {
            let parsed: Ipv4Addr = ip.parse().map_err(|_| "Invalid interface IP")?;
            SocketAddrV4::new(parsed, 0)
        } else {
            SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)
        };

        let sock_bind = SockAddr::from(bind_addr);
        socket.bind(&sock_bind).map_err(|e| format!("Failed to bind: {}", e))?;

        if cfg!(unix) {
            socket.set_multicast_loop_v4(true).ok();
            socket.set_multicast_ttl_v4(64).ok();
        } else {
            socket.set_multicast_ttl_v4(64).ok();
        }

        let std_socket: UdpSocket = socket.into();
        std_socket.set_nonblocking(true).map_err(|e| e.to_string())?;

        self.local_addr = std_socket.local_addr().ok().and_then(|a| {
            if let std::net::SocketAddr::V4(v4) = a {
                Some(v4)
            } else {
                None
            }
        });
        self.socket = Some(std_socket);
        self.enabled = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.socket.take();
        self.enabled = false;
    }

    pub fn broadcast(&mut self, msg: &PositionBroadcast) -> Result<usize, String> {
        if !self.enabled { return Err("Not started".into()); }
        let socket = self.socket.as_ref().ok_or("No socket")?;
        let seq = self.sequence.fetch_add(1, Ordering::AcqRel);
        let mut out_msg = *msg;
        out_msg.session_id = self.session_id;
        out_msg.sequence = seq;
        let bytes = out_msg.to_bytes();
        socket.send_to(&bytes, self.multicast_addr)
            .map_err(|e| e.to_string())
    }

    pub fn broadcast_interval(&self) -> Duration {
        Duration::from_millis(BROADCAST_INTERVAL_MS)
    }

    pub fn is_enabled(&self) -> bool { self.enabled }

    pub fn session_id(&self) -> u64 { self.session_id }

    pub fn multicast_addr(&self) -> SocketAddrV4 { self.multicast_addr }
}

pub struct MulticastReceiver {
    socket: Option<UdpSocket>,
    buffer: [u8; 2048],
    enabled: bool,
}

impl MulticastReceiver {
    pub fn new() -> Self {
        Self { socket: None, buffer: [0u8; 2048], enabled: false }
    }

    pub fn start(&mut self, multicast_ip: &str, port: u16, interface_ip: Option<&str>) -> Result<(), String> {
        let group_ip: Ipv4Addr = multicast_ip.parse().map_err(|_| "Invalid multicast IP")?;
        let interface: Ipv4Addr = interface_ip
            .map(|s| s.parse().map_err(|_| "Invalid interface IP"))
            .unwrap_or(Ok(Ipv4Addr::UNSPECIFIED))?;

        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
            .map_err(|e| e.to_string())?;
        socket.join_multicast_v4(&group_ip, &interface)
            .map_err(|e| format!("Failed to join multicast: {}", e))?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        self.socket = Some(socket);
        self.enabled = true;
        Ok(())
    }

    pub fn recv(&mut self) -> Option<(PositionBroadcast, std::net::SocketAddr)> {
        if !self.enabled { return None; }
        let socket = self.socket.as_ref()?;
        match socket.recv_from(&mut self.buffer) {
            Ok((n, addr)) if n >= 41 => {
                let msg = PositionBroadcast::from_bytes(&self.buffer[..n])?;
                Some((msg, addr))
            }
            _ => None,
        }
    }

    pub fn stop(&mut self) { self.socket.take(); self.enabled = false; }
}

impl Default for MulticastReceiver { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_serde() {
        let mut b = PositionBroadcast::default();
        b.position_seconds = 3661.5;
        b.timecode_hh = 1;
        b.timecode_mm = 1;
        b.timecode_ss = 1;
        b.timecode_ff = 15;
        let bytes = b.to_bytes();
        let back = PositionBroadcast::from_bytes(&bytes).unwrap();
        assert_eq!(back.timecode_hh, 1);
        assert_eq!(back.position_seconds, 3661.5);
    }
}
