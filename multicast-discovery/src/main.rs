use futures::lock::Mutex;
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use socket2::Socket;
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};

// --- CONSTANTS AND CONFIGURATION ---

// Multicast group address for initial node discovery (FIXED PORT)
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 0, 0, 1);
// Fixed port used for sending/receiving MULTICAST DISCOVERY messages.
const MULTICAST_PORT: u16 = 5000;

// IMPORTANT CHANGE: We now bind to 127.0.0.1 and let the OS pick a unique port (Port 0)
// This is essential for running multiple nodes on the same host.
const LOCAL_INTERFACE_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

// --- DATA STRUCTURES ---

// A unique identifier for a node
type NodeId = u32;

/// Information about a single node in the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    // The unique, dynamically assigned address for this node's UNICAST GOSSIP/LSP messages.
    pub addr: SocketAddr,
    pub known_neighbors: HashSet<NodeId>,
    pub last_update_timestamp: u64,
}

/// The state of the entire distributed network topology.
type NetworkState = HashMap<NodeId, NodeInfo>;

/// Messages exchanged between nodes via UDP
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    /// Sent to the multicast group for initial discovery.
    Discovery(NodeInfo),
    /// Unicast message for status confirmation and topology sharing (Gossip).
    Gossip(NodeInfo),
    /// Sent when a new link state is confirmed/detected (LSP update).
    LinkStateUpdate(NodeInfo),
}

// --- CORE LOGIC ---

/// Binds the UDP socket to a unique ephemeral port, joins the multicast group, and sets time-to-live.
/// Returns the bound socket and its actual local address.
async fn setup_udp_socket(_local_id: NodeId) -> (UdpSocket, UdpSocket, SocketAddr) {
    // 1. Bind to a dynamic port (Port 0) on the loopback interface (127.0.0.1)
    let socket_addr = SocketAddrV4::new(LOCAL_INTERFACE_IP, 0);

    // 3. Join the multicast group on the FIXED port (MULTICAST_PORT)
    let socket = Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )
    .unwrap();
    socket.set_reuse_address(true).unwrap();
    socket.set_reuse_port(true).unwrap();
    socket.set_multicast_loop_v4(true).unwrap();
    socket.set_nonblocking(true).unwrap();

    socket.bind(&socket_addr.into()).unwrap();
    socket
        .join_multicast_v4(&MULTICAST_ADDR, &LOCAL_INTERFACE_IP)
        .unwrap();

    let tokio_socket = UdpSocket::from_std(socket.into()).unwrap();
    let actual_local_addr = tokio_socket.local_addr().unwrap();
    let mcast_socket = {
        let socket_addr = SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT);
        let socket = Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        socket.set_reuse_address(true).unwrap();
        socket.set_reuse_port(true).unwrap();
        socket.set_multicast_loop_v4(true).unwrap();
        socket.set_nonblocking(true).unwrap();

        socket.bind(&socket_addr.into()).unwrap();
        socket
            .join_multicast_v4(&MULTICAST_ADDR, &LOCAL_INTERFACE_IP)
            .unwrap();
        UdpSocket::from_std(socket.into()).unwrap()
    };

    (tokio_socket, mcast_socket, actual_local_addr)
}

/// Task 1: Handles incoming UDP messages (Discovery, Gossip, LinkStateUpdate).
async fn state_listener(
    socket: Arc<UdpSocket>,
    network_state: Arc<Mutex<NetworkState>>,
    local_id: NodeId,
) {
    let mut buf = vec![0u8; 1024];

    println!("[Node {}] State listener started.", local_id);

    loop {
        // This socket listens for ALL traffic (multicast discovery AND unicast gossip)
        match socket.recv_from(&mut buf).await {
            Ok((len, _sender_addr)) => {
                //println!("receive {len} bytes from {sender_addr}");
                let data = &buf[..len];
                let msg: Result<(Message, usize), _> =
                    bincode::serde::decode_from_slice(data, bincode::config::standard());

                match msg {
                    Ok((message, _)) => {
                        // Get the sender's info (ID and unicast address)
                        let sender_info = match &message {
                            Message::Discovery(info)
                            | Message::Gossip(info)
                            | Message::LinkStateUpdate(info) => info,
                        };

                        if sender_info.id == local_id {
                            continue; // Ignore messages sent by ourselves
                        }

                        let mut state = network_state.lock().await;

                        match message {
                            Message::Discovery(info) => {
                                // 1. Node Discovery: Initial connection (Multicast)
                                if state.contains_key(&info.id) {
                                    // Update the known address in case the remote node restarted on a new port
                                    if let Some(existing_info) = state.get_mut(&info.id) {
                                        existing_info.addr = info.addr;
                                    }
                                    // println!("[Node {}] Discovery from existing node: {}", local_id, info.id);
                                } else {
                                    println!(
                                        "[Node {}] Discovered NEW node: {} at {}",
                                        local_id, info.id, info.addr
                                    );
                                }
                                state.insert(info.id, info);
                            }
                            Message::Gossip(remote_info)
                            | Message::LinkStateUpdate(remote_info) => {
                                // 2. Gossip/LSP: Update existing state (Unicast)
                                let current_info = state.get_mut(&remote_info.id);

                                if let Some(local_info) = current_info {
                                    if remote_info.last_update_timestamp
                                        > local_info.last_update_timestamp
                                    {
                                        // Update local node state with fresher remote data
                                        println!(
                                            "[Node {}] FSM update from {}. Links: {:?}",
                                            local_id, remote_info.id, remote_info.known_neighbors
                                        );
                                        local_info.last_update_timestamp =
                                            remote_info.last_update_timestamp;
                                        // Merge known neighbors (the core of LSP - sharing the link state)
                                        local_info
                                            .known_neighbors
                                            .extend(remote_info.known_neighbors.into_iter());
                                        local_info.addr = remote_info.addr; // Ensure address is up-to-date
                                    }
                                } else {
                                    // If we received gossip from an unknown node, insert it (discovery fallback)
                                    println!(
                                        "[Node {}] Gossip/LSP from UNKNOWN node: {} at {}",
                                        local_id, remote_info.id, remote_info.addr
                                    );
                                    state.insert(remote_info.id, remote_info);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Ignore non-deserializable packets (e.g., from other apps using the same multicast group)
                        eprintln!("[Node {}] Failed to deserialize message: {}", local_id, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[Node {}] UDP receive error: {}", local_id, e);
            }
        }
    }
}

/// Task 2: Periodically sends the node's info to the multicast address for new nodes to discover it.
async fn discovery_beacon(
    socket: Arc<UdpSocket>,
    network_state: Arc<Mutex<NetworkState>>,
    local_id: NodeId,
) {
    // Multicast destination uses the FIXED port
    let multicast_dest = SocketAddr::V4(SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT));
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let state = network_state.lock().await;
        // The local_info now contains the unique, dynamically assigned port
        let local_info = state.get(&local_id).expect("Local node info must exist.");
        let msg = Message::Discovery(local_info.clone());

        match bincode::serde::encode_to_vec(&msg, bincode::config::standard()) {
            Ok(bytes) => {
                // Send the Discovery message to the Multicast Group
                if let Err(e) = socket.send_to(&bytes, multicast_dest).await {
                    eprintln!("[Node {}] Failed to send discovery beacon: {}", local_id, e);
                }
            }
            Err(e) => eprintln!(
                "[Node {}] Failed to serialize discovery message: {}",
                local_id, e
            ),
        }
    }
}

/// Task 3: Periodically initiates gossip with a random subset of known nodes (UNICAST).
/// This confirms links and distributes Link State Updates (LSP).
async fn gossip_loop(
    socket: Arc<UdpSocket>,
    network_state: Arc<Mutex<NetworkState>>,
    local_id: NodeId,
) {
    let mut interval = time::interval(Duration::from_secs(2));
    const GOSSIP_FANOUT: usize = 3; // Number of nodes to gossip with per cycle

    loop {
        interval.tick().await;

        let state = network_state.lock().await;
        let local_info = state.get(&local_id).expect("Local node info must exist.");

        // Create the message to gossip (Gossip or LinkStateUpdate are similar here)
        let msg = Message::Gossip(local_info.clone());
        let bytes = match bincode::serde::encode_to_vec(&msg, bincode::config::standard()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "[Node {}] Failed to serialize gossip message: {}",
                    local_id, e
                );
                continue;
            }
        };

        // Select a random set of known nodes (excluding self) to gossip with
        let peers: Vec<SocketAddr> = state
            .iter()
            .filter(|(id, _)| **id != local_id)
            .map(|(_, info)| info.addr)
            .choose_multiple(&mut rand::rng(), GOSSIP_FANOUT);

        if peers.is_empty() {
            continue;
        }

        // Send the gossip message to selected peers (Unicast UDP to their unique dynamic port)
        for peer_addr in peers {
            if let Err(e) = socket.send_to(&bytes, peer_addr).await {
                // In a real system, a failure to send would trigger a link failure detection.
                eprintln!(
                    "[Node {}] Failed to gossip to {}: {}",
                    local_id, peer_addr, e
                );
            }
        }
    }
}

/// Task 4: Simple diagnostic and Link-State Path Calculation (Conceptual)
async fn diagnostics_and_lsp_calculator(network_state: Arc<Mutex<NetworkState>>, local_id: NodeId) {
    let mut interval = time::interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        let state = network_state.lock().await;
        println!(
            "\n--- [Node {}] Network Topology ({}) ---",
            local_id,
            state.len()
        );

        for (id, info) in state.iter() {
            println!(
                "  Node {}: Unicast Addr={}, Links={:?}",
                id, info.addr, info.known_neighbors
            );
        }
        println!("--------------------------------------\n");

        // The NetworkState contains the full LSA database required to run
        // Dijkstra's algorithm for path finding.
    }
}

// --- MAIN APPLICATION ---

#[tokio::main]
async fn main() {
    // 1. Get a unique ID for this instance (e.g., from command line or config file)
    let args: Vec<String> = std::env::args().collect();
    let local_id: NodeId = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

    // 2. Setup Socket and get the dynamically assigned port
    let (socket, mcast_socket, actual_local_addr) = setup_udp_socket(local_id).await;
    let socket = Arc::new(socket);
    let mcast_socket = Arc::new(mcast_socket);

    // 3. Initialize Shared State using the actual dynamic address
    let network_state = Arc::new(Mutex::new(NetworkState::new()));

    // Insert the local node into the network state initially
    let initial_info = NodeInfo {
        id: local_id,
        addr: actual_local_addr,
        // For demonstration, let's assume Node 1 initially knows Node 2 as a neighbor (link)
        known_neighbors: HashSet::from([local_id]),
        last_update_timestamp: 1,
    };
    network_state.lock().await.insert(local_id, initial_info);

    // 4. Spawn Concurrent Tasks
    tokio::spawn(discovery_beacon(
        Arc::clone(&socket),
        Arc::clone(&network_state),
        local_id,
    ));
    tokio::spawn(gossip_loop(
        Arc::clone(&socket),
        Arc::clone(&network_state),
        local_id,
    ));
    tokio::spawn(state_listener(
        Arc::clone(&socket),
        Arc::clone(&network_state),
        local_id,
    ));
    tokio::spawn(state_listener(
        Arc::clone(&mcast_socket),
        Arc::clone(&network_state),
        local_id,
    ));
    tokio::spawn(diagnostics_and_lsp_calculator(
        Arc::clone(&network_state),
        local_id,
    ));

    // 5. Keep main thread alive
    std::future::pending::<()>().await;
}
