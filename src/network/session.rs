use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use serde::{Deserialize, Serialize};

use crate::ecs::world::World;
use crate::network::snapshot::WorldSnapshot;
use crate::protocol::commands::Command;

const PROTOCOL_VERSION: u16 = 1;
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(100);
const INPUT_INTERVAL: Duration = Duration::from_millis(50);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(12);
pub(crate) const SNAPSHOT_CHUNK_PAYLOAD: usize = 1100;
const CHUNK_PAYLOAD: usize = SNAPSHOT_CHUNK_PAYLOAD;
const MAX_SNAPSHOT_BYTES: usize = 2_000_000;
const MAX_DECOMPRESSED_SNAPSHOT_BYTES: usize = 8_000_000;

pub(crate) fn decode_snapshot_payload(compressed: &[u8]) -> Option<WorldSnapshot> {
    if compressed.len() > MAX_SNAPSHOT_BYTES {
        return None;
    }
    let mut decoder = ZlibDecoder::new(compressed)
        .take((MAX_DECOMPRESSED_SNAPSHOT_BYTES + 1) as u64);
    let mut json = Vec::new();
    if decoder.read_to_end(&mut json).is_err() || json.len() > MAX_DECOMPRESSED_SNAPSHOT_BYTES {
        return None;
    }
    serde_json::from_slice::<WorldSnapshot>(&json).ok()
}

#[derive(Serialize, Deserialize)]
enum ControlPacket {
    Hello { version: u16, name: String, skin: u8 },
    Welcome { version: u16, player_id: u8 },
    Command { version: u16, command: Command },
}

struct Peer {
    id: u8,
    name: String,
    skin: u8,
    last_seen: Instant,
    disconnected: bool,
}

struct PendingSnapshot {
    chunks: Vec<Option<Vec<u8>>>,
    received: usize,
    created: Instant,
}

enum SessionMode {
    Local { inbox: Vec<Command> },
    Host {
        socket: UdpSocket,
        peers: HashMap<SocketAddr, Peer>,
        inbox: Vec<Command>,
        last_snapshot: Instant,
        sequence: u32,
    },
    Client {
        socket: UdpSocket,
        host: SocketAddr,
        name: String,
        skin: u8,
        player_id: Option<u8>,
        last_hello: Instant,
        last_input: Instant,
        pending_input: Option<Command>,
        pending_dash: bool,
        pending_snapshots: HashMap<u32, PendingSnapshot>,
        latest_snapshot: Option<WorldSnapshot>,
        last_sequence: Option<u32>,
        last_snapshot_received: Option<Instant>,
    },
}

pub struct UdpDriver {
    mode: SessionMode,
}

impl UdpDriver {
    pub fn local() -> Self {
        Self { mode: SessionMode::Local { inbox: Vec::new() } }
    }

    pub fn host(bind: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind)?;
        socket.set_nonblocking(true)?;
        println!("UDP host listening on {}", socket.local_addr()?);
        Ok(Self {
            mode: SessionMode::Host {
                socket,
                peers: HashMap::new(),
                inbox: Vec::new(),
                last_snapshot: Instant::now(),
                sequence: 0,
            },
        })
    }

    pub fn client(host: SocketAddr, name: String, skin: u8) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            mode: SessionMode::Client {
                socket,
                host,
                name: name.chars().take(32).collect(),
                skin: skin % 5,
                player_id: None,
                last_hello: Instant::now() - Duration::from_secs(2),
                last_input: Instant::now(),
                pending_input: None,
                pending_dash: false,
                pending_snapshots: HashMap::new(),
                latest_snapshot: None,
                last_sequence: None,
                last_snapshot_received: None,
            },
        })
    }

    pub fn is_client(&self) -> bool { matches!(self.mode, SessionMode::Client { .. }) }
    pub fn is_host(&self) -> bool { matches!(self.mode, SessionMode::Host { .. }) }
    pub fn is_networked(&self) -> bool { !matches!(self.mode, SessionMode::Local { .. }) }

    #[cfg(test)]
    pub(crate) fn host_local_addr(&self) -> Option<SocketAddr> {
        match &self.mode {
            SessionMode::Host { socket, .. } => socket.local_addr().ok(),
            _ => None,
        }
    }

    pub fn player_id(&self) -> Option<u8> {
        match &self.mode {
            SessionMode::Client { player_id, .. } => *player_id,
            _ => None,
        }
    }

    pub fn is_connected(&self) -> bool {
        match &self.mode {
            SessionMode::Local { .. } | SessionMode::Host { .. } => true,
            SessionMode::Client { player_id, last_snapshot_received, .. } => {
                player_id.is_some() && last_snapshot_received
                    .is_some_and(|received| received.elapsed() < Duration::from_secs(3))
            }
        }
    }

    pub fn take_snapshot(&mut self) -> Option<WorldSnapshot> {
        match &mut self.mode {
            SessionMode::Client { latest_snapshot, .. } => latest_snapshot.take(),
            _ => None,
        }
    }

    pub fn publish_snapshot(&mut self, world: &World) {
        let SessionMode::Host { socket, peers, last_snapshot, sequence, .. } = &mut self.mode else {
            return;
        };
        let now = Instant::now();
        if now.duration_since(*last_snapshot) < SNAPSHOT_INTERVAL { return; }
        *last_snapshot = now;
        *sequence = sequence.wrapping_add(1);
        let snapshot = WorldSnapshot::from_world(world, *sequence);
        let Ok(json) = serde_json::to_vec(&snapshot) else { return; };
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        if encoder.write_all(&json).is_err() { return; }
        let Ok(compressed) = encoder.finish() else { return; };
        if compressed.len() > MAX_SNAPSHOT_BYTES {
            eprintln!("UDP snapshot too large ({} bytes), skipping frame", compressed.len());
            return;
        }
        let count = compressed.len().div_ceil(CHUNK_PAYLOAD);
        let count = count.min(u16::MAX as usize);
        for (addr, peer) in peers.iter().filter(|(_, peer)| !peer.disconnected) {
            let _ = peer.id;
            for chunk_index in 0..count {
                let start = chunk_index * CHUNK_PAYLOAD;
                let end = (start + CHUNK_PAYLOAD).min(compressed.len());
                let mut packet = Vec::with_capacity(12 + end - start);
                packet.extend_from_slice(b"AVS1");
                packet.extend_from_slice(&sequence.to_be_bytes());
                packet.extend_from_slice(&(chunk_index as u16).to_be_bytes());
                packet.extend_from_slice(&(count as u16).to_be_bytes());
                packet.extend_from_slice(&compressed[start..end]);
                let _ = socket.send_to(&packet, addr);
            }
        }
    }

    fn poll_host(socket: &UdpSocket, peers: &mut HashMap<SocketAddr, Peer>, inbox: &mut Vec<Command>) {
        let mut buffer = [0u8; 65_535];
        loop {
            let Ok((length, addr)) = socket.recv_from(&mut buffer) else { break; };
            let bytes = &buffer[..length];
            if bytes.starts_with(b"AVS1") { continue; }
            let Ok(packet) = serde_json::from_slice::<ControlPacket>(bytes) else { continue; };
            match packet {
                ControlPacket::Hello { version, name, skin } if version == PROTOCOL_VERSION => {
                    let id = if let Some(peer) = peers.get_mut(&addr) {
                        let was_disconnected = peer.disconnected;
                        peer.last_seen = Instant::now();
                        peer.disconnected = false;
                        if was_disconnected {
                            inbox.push(Command::JoinBoss { player_id: peer.id, name: peer.name.clone(), skin: peer.skin });
                        }
                        peer.id
                    } else {
                        let used: Vec<u8> = peers.values().map(|peer| peer.id).collect();
                        let Some(id) = (1u8..=8).find(|id| !used.contains(id)) else { continue; };
                        let player_name: String = name.chars().take(32).collect();
                        let player_skin = skin % 5;
                        peers.insert(addr, Peer {
                            id,
                            name: player_name.clone(),
                            skin: player_skin,
                            last_seen: Instant::now(),
                            disconnected: false,
                        });
                        inbox.push(Command::JoinBoss { player_id: id, name: player_name, skin: player_skin });
                        id
                    };
                    let welcome = ControlPacket::Welcome { version: PROTOCOL_VERSION, player_id: id };
                    if let Ok(payload) = serde_json::to_vec(&welcome) { let _ = socket.send_to(&payload, addr); }
                }
                ControlPacket::Command { version, command } if version == PROTOCOL_VERSION => {
                    let cmd_player_id = match &command {
                        Command::BossInput { player_id, .. }
                        | Command::SelectPerk { player_id, .. }
                        | Command::TogglePause { player_id, .. } => Some(*player_id),
                        _ => None,
                    };
                    if let (Some(player_id), Some(peer)) = (cmd_player_id, peers.get_mut(&addr)) {
                        peer.last_seen = Instant::now();
                        // Any validated peer packet can arrive before the periodic Hello after
                        // a timeout. Restore human control here so this packet cannot
                        // consume the disconnected flag and leave the actor AI-driven.
                        if peer.disconnected {
                            inbox.push(Command::JoinBoss {
                                player_id: peer.id,
                                name: peer.name.clone(),
                                skin: peer.skin,
                            });
                        }
                        peer.disconnected = false;
                        if peer.id == player_id {
                            inbox.push(command);
                        }
                    }
                }
                _ => {}
            }
        }

        let now = Instant::now();
        for peer in peers.values_mut() {
            if !peer.disconnected && now.duration_since(peer.last_seen) > CLIENT_TIMEOUT {
                peer.disconnected = true;
                inbox.push(Command::DisconnectBoss { player_id: peer.id });
            }
        }
    }

    fn poll_client(
        socket: &UdpSocket,
        host: SocketAddr,
        name: &str,
        skin: u8,
        player_id: &mut Option<u8>,
        last_hello: &mut Instant,
        last_input: &mut Instant,
        pending_input: &mut Option<Command>,
        pending_dash: &mut bool,
        pending_snapshots: &mut HashMap<u32, PendingSnapshot>,
        latest_snapshot: &mut Option<WorldSnapshot>,
        last_sequence: &mut Option<u32>,
        last_snapshot_received: &mut Option<Instant>,
    ) {
        let now = Instant::now();
        if (player_id.is_none() && now.duration_since(*last_hello) >= Duration::from_millis(700))
            || (player_id.is_some() && now.duration_since(*last_hello) >= Duration::from_secs(2))
        {
            *last_hello = now;
            let hello = ControlPacket::Hello { version: PROTOCOL_VERSION, name: name.to_string(), skin };
            if let Ok(bytes) = serde_json::to_vec(&hello) { let _ = socket.send_to(&bytes, host); }
        }
        if now.duration_since(*last_input) >= INPUT_INTERVAL {
            if let Some(mut command) = pending_input.take() {
                let should_send = if let Command::BossInput { player_id: id, dash_pressed, .. } = &mut command {
                    if let Some(assigned) = *player_id {
                        *id = assigned;
                        *dash_pressed |= *pending_dash;
                        *pending_dash = false;
                        true
                    } else {
                        false
                    }
                } else { false };
                if should_send {
                    let packet = ControlPacket::Command { version: PROTOCOL_VERSION, command };
                    if let Ok(bytes) = serde_json::to_vec(&packet) { let _ = socket.send_to(&bytes, host); }
                }
            }
            *last_input = now;
        }

        let mut buffer = [0u8; 65_535];
        loop {
            let Ok((length, addr)) = socket.recv_from(&mut buffer) else { break; };
            if addr != host { continue; }
            let bytes = &buffer[..length];
            if bytes.starts_with(b"AVS1") {
                if bytes.len() < 12 { continue; }
                let sequence = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
                let index = u16::from_be_bytes(bytes[8..10].try_into().unwrap()) as usize;
                let count = u16::from_be_bytes(bytes[10..12].try_into().unwrap()) as usize;
                if count == 0 || count > 2048 || index >= count { continue; }
                if let Some(previous) = *last_sequence {
                    if sequence != previous && sequence.wrapping_sub(previous) > u32::MAX / 2 { continue; }
                }
                let pending = pending_snapshots.entry(sequence).or_insert_with(|| PendingSnapshot {
                    chunks: vec![None; count], received: 0, created: now,
                });
                if pending.chunks.len() != count { pending_snapshots.remove(&sequence); continue; }
                if pending.chunks[index].is_none() {
                    pending.chunks[index] = Some(bytes[12..].to_vec());
                    pending.received += 1;
                }
                if pending.received == count {
                    let pending = pending_snapshots.remove(&sequence).unwrap();
                    let mut compressed = Vec::new();
                    for chunk in pending.chunks.into_iter().flatten() { compressed.extend_from_slice(&chunk); }
                    if let Some(snapshot) = decode_snapshot_payload(&compressed) {
                        *last_sequence = Some(sequence);
                        *latest_snapshot = Some(snapshot);
                        *last_snapshot_received = Some(Instant::now());
                    }
                }
            } else if let Ok(ControlPacket::Welcome { version, player_id: id }) = serde_json::from_slice::<ControlPacket>(bytes) {
                if version == PROTOCOL_VERSION && (1..=8).contains(&id) { *player_id = Some(id); }
            }
        }
        pending_snapshots.retain(|_, pending| now.duration_since(pending.created) < Duration::from_secs(2));
    }
}

impl super::driver::NetworkDriver for UdpDriver {
    fn send_command(&mut self, command: Command) {
        match &mut self.mode {
            SessionMode::Local { inbox } | SessionMode::Host { inbox, .. } => inbox.push(command),
            SessionMode::Client { socket, host, player_id, pending_input, pending_dash, .. } => {
                match &command {
                    Command::BossInput { dash_pressed, .. } => {
                        *pending_dash |= *dash_pressed;
                        *pending_input = Some(command);
                    }
                    Command::SelectPerk { .. } | Command::TogglePause { .. } => {
                        if player_id.is_some() {
                            let packet = ControlPacket::Command { version: PROTOCOL_VERSION, command };
                            if let Ok(bytes) = serde_json::to_vec(&packet) {
                                let _ = socket.send_to(&bytes, *host);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn poll_commands(&mut self) -> Vec<Command> {
        match &mut self.mode {
            SessionMode::Local { inbox } => std::mem::take(inbox),
            SessionMode::Host { socket, peers, inbox, .. } => {
                Self::poll_host(socket, peers, inbox);
                std::mem::take(inbox)
            }
            SessionMode::Client {
                socket, host, name, skin, player_id, last_hello, last_input,
                pending_input, pending_dash, pending_snapshots, latest_snapshot, last_sequence,
                last_snapshot_received,
            } => {
                Self::poll_client(socket, *host, name, *skin, player_id, last_hello,
                    last_input, pending_input, pending_dash, pending_snapshots,
                    latest_snapshot, last_sequence, last_snapshot_received);
                Vec::new()
            }
        }
    }

    fn is_network_client(&self) -> bool { self.is_client() }
    fn is_network_host(&self) -> bool { self.is_host() }
    fn is_networked(&self) -> bool { UdpDriver::is_networked(self) }
    fn is_network_connected(&self) -> bool { UdpDriver::is_connected(self) }
    fn assigned_player_id(&self) -> Option<u8> { self.player_id() }
    fn take_snapshot(&mut self) -> Option<WorldSnapshot> { UdpDriver::take_snapshot(self) }
    fn publish_snapshot(&mut self, world: &World) { UdpDriver::publish_snapshot(self, world); }
    fn rejoin_commands(&self) -> Vec<Command> {
        match &self.mode {
            SessionMode::Host { peers, .. } => peers.values()
                .filter(|peer| !peer.disconnected)
                .map(|peer| Command::JoinBoss { player_id: peer.id, name: peer.name.clone(), skin: peer.skin })
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::driver::NetworkDriver;

    #[test]
    fn test_decode_snapshot_payload_rejects_oversized_zlib_bomb() {
        let oversized = vec![b'A'; MAX_DECOMPRESSED_SNAPSHOT_BYTES + 4096];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&oversized).expect("compress oversized test buffer");
        let compressed = encoder.finish().expect("finish zlib stream");
        assert!(compressed.len() <= MAX_SNAPSHOT_BYTES);
        assert!(decode_snapshot_payload(&compressed).is_none());
    }

    #[test]
    fn test_udp_host_accepts_select_perk_and_toggle_pause_and_blocks_spoofing() {
        let mut host = UdpDriver::host("127.0.0.1:0").expect("bind UDP host");
        let host_addr = host.host_local_addr().expect("host addr");
        let mut client = UdpDriver::client(host_addr, "TestBoss".to_string(), 1)
            .expect("bind UDP client");

        // Handshake Hello -> Welcome
        let mut joined = false;
        let mut host_cmds = Vec::new();
        for _ in 0..50 {
            let _ = client.poll_commands();
            host_cmds.extend(host.poll_commands());
            let _ = client.poll_commands();
            if client.assigned_player_id() == Some(1)
                && host_cmds.iter().any(|c| matches!(c, Command::JoinBoss { player_id: 1, .. }))
            {
                joined = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(joined, "Client should complete Hello/Welcome handshake");

        // Валидные команды клиента + попытка спуфинга чужого ID и запрещённой хостовой команды
        client.send_command(Command::SelectPerk { player_id: 1, choice_index: 2 });
        client.send_command(Command::TogglePause { player_id: 1, paused: true });
        client.send_command(Command::SelectPerk { player_id: 2, choice_index: 0 });
        if let SessionMode::Client { socket, host: target, .. } = &client.mode {
            let spoofed_host_cmd = ControlPacket::Command {
                version: PROTOCOL_VERSION,
                command: Command::SpawnSquad { director_id: 0, tier: 1 },
            };
            let bytes = serde_json::to_vec(&spoofed_host_cmd).unwrap();
            let _ = socket.send_to(&bytes, *target);
        }

        let mut received = Vec::new();
        for _ in 0..50 {
            received.extend(host.poll_commands());
            if received.len() >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(received.iter().any(|c| matches!(c, Command::SelectPerk { player_id: 1, choice_index: 2 })));
        assert!(received.iter().any(|c| matches!(c, Command::TogglePause { player_id: 1, paused: true })));
        assert!(!received.iter().any(|c| matches!(c, Command::SelectPerk { player_id: 2, .. })));
        assert!(!received.iter().any(|c| matches!(c, Command::SpawnSquad { .. })));
    }
}
