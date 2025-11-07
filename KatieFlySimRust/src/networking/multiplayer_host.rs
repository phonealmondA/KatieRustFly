// Multiplayer Host - Authoritative server implementation
// Phase 11: Multiplayer Host

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::networking::{
    NetworkManager, NetworkMessage, NetworkError,
    PlayerStateData, GameStateSnapshot,
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
const GAME_STATE_UPDATE_RATE: f32 = 1.0 / 30.0; // 30 updates per second

/// Connected client information
#[derive(Debug)]
struct ConnectedClient {
    player_id: u32,
    addr: SocketAddr,
    player_name: String,
    last_heartbeat: Instant,
    tx: mpsc::UnboundedSender<NetworkMessage>,
}

/// Host events for game integration
#[derive(Debug, Clone)]
pub enum HostEvent {
    ClientConnected { player_id: u32, player_name: String },
    ClientDisconnected { player_id: u32 },
    PlayerInput { player_id: u32, thrust: bool, rotate_left: bool, rotate_right: bool, launch: bool, convert_to_satellite: bool },
}

/// Multiplayer Host with authoritative server
pub struct MultiplayerHost {
    running: bool,
    port: u16,
    next_player_id: u32,

    // Client management
    clients: Arc<Mutex<HashMap<u32, ConnectedClient>>>,

    // Event channels
    event_tx: mpsc::UnboundedSender<HostEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<HostEvent>>,

    // Game state
    current_frame: u64,
    game_time: f64,
    update_accumulator: f32,

    // Background tasks
    listener_task: Option<JoinHandle<()>>,
    heartbeat_task: Option<JoinHandle<()>>,
}

impl MultiplayerHost {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        MultiplayerHost {
            running: false,
            port: 0,
            next_player_id: 1,
            clients: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            event_rx: Some(event_rx),
            current_frame: 0,
            game_time: 0.0,
            update_accumulator: 0.0,
            listener_task: None,
            heartbeat_task: None,
        }
    }

    /// Start hosting on specified port
    pub fn start(&mut self, port: u16) -> Result<(), String> {
        if self.running {
            return Err("Host already running".to_string());
        }

        self.port = port;
        self.running = true;

        // Spawn listener task
        let clients = Arc::clone(&self.clients);
        let event_tx = self.event_tx.clone();
        let next_player_id = Arc::new(Mutex::new(self.next_player_id));

        let listener_task = tokio::spawn(async move {
            if let Err(e) = Self::run_listener(port, clients, event_tx, next_player_id).await {
                eprintln!("Listener error: {}", e);
            }
        });

        self.listener_task = Some(listener_task);

        // Spawn heartbeat task
        let clients = Arc::clone(&self.clients);
        let heartbeat_task = tokio::spawn(async move {
            Self::run_heartbeat_monitor(clients).await;
        });

        self.heartbeat_task = Some(heartbeat_task);

        Ok(())
    }

    /// Stop hosting
    pub fn stop(&mut self) {
        self.running = false;

        // Disconnect all clients
        if let Ok(mut clients) = self.clients.lock() {
            for (_, client) in clients.iter() {
                let _ = client.tx.send(NetworkMessage::Disconnect {
                    reason: "Server shutting down".to_string(),
                });
            }
            clients.clear();
        }

        // Abort background tasks
        if let Some(task) = self.listener_task.take() {
            task.abort();
        }
        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
    }

    /// Update host state
    pub fn update(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }

        self.game_time += delta_time as f64;
        self.update_accumulator += delta_time;

        // Send game state updates at fixed rate
        if self.update_accumulator >= GAME_STATE_UPDATE_RATE {
            self.update_accumulator = 0.0;
            self.current_frame += 1;

            // Broadcast game state to all clients
            self.broadcast_game_state();
        }
    }

    /// Broadcast message to all clients
    pub fn broadcast(&mut self, message: NetworkMessage) {
        if let Ok(clients) = self.clients.lock() {
            for (_, client) in clients.iter() {
                let _ = client.tx.send(message.clone());
            }
        }
    }

    /// Broadcast game state update
    fn broadcast_game_state(&self) {
        let players = if let Ok(clients) = self.clients.lock() {
            clients.iter().map(|(player_id, client)| {
                PlayerStateData {
                    player_id: *player_id,
                    position: (0.0, 0.0), // TODO: Get actual player position from game world
                    velocity: (0.0, 0.0),
                    rotation: 0.0,
                    fuel: 100.0,
                    is_alive: true,
                }
            }).collect()
        } else {
            Vec::new()
        };

        let message = NetworkMessage::GameStateUpdate {
            frame: self.current_frame,
            timestamp: self.game_time,
            players,
        };

        if let Ok(clients) = self.clients.lock() {
            for (_, client) in clients.iter() {
                let _ = client.tx.send(message.clone());
            }
        }
    }

    /// Get host events
    pub fn poll_events(&mut self) -> Vec<HostEvent> {
        let mut events = Vec::new();

        if let Some(rx) = &mut self.event_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        events
    }

    /// Run TCP listener (async task)
    async fn run_listener(
        port: u16,
        clients: Arc<Mutex<HashMap<u32, ConnectedClient>>>,
        event_tx: mpsc::UnboundedSender<HostEvent>,
        next_player_id: Arc<Mutex<u32>>,
    ) -> Result<(), NetworkError> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        println!("Multiplayer host listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let player_id = {
                        let mut next_id = next_player_id.lock().unwrap();
                        let id = *next_id;
                        *next_id += 1;
                        id
                    };

                    let clients = Arc::clone(&clients);
                    let event_tx = event_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, addr, player_id, clients, event_tx).await {
                            eprintln!("Client {} error: {}", player_id, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    }

    /// Handle individual client connection (async task)
    async fn handle_client(
        mut stream: TcpStream,
        addr: SocketAddr,
        player_id: u32,
        clients: Arc<Mutex<HashMap<u32, ConnectedClient>>>,
        event_tx: mpsc::UnboundedSender<HostEvent>,
    ) -> Result<(), NetworkError> {
        // Receive handshake
        let handshake = NetworkManager::receive_message_async(&mut stream).await?;

        let player_name = match handshake {
            NetworkMessage::Handshake { version: _, player_name } => player_name,
            _ => return Err(NetworkError::ConnectionFailed("Expected handshake".to_string())),
        };

        println!("Player {} ({}) connected from {}", player_id, player_name, addr);

        // Send handshake acknowledgment
        let game_state = GameStateSnapshot {
            frame: 0,
            players: Vec::new(),
        };

        let ack = NetworkMessage::HandshakeAck {
            player_id,
            game_state,
        };

        NetworkManager::send_message_async(&mut stream, &ack).await?;

        // Create message channel for this client
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Add client to connected clients
        {
            let mut clients_lock = clients.lock().unwrap();
            clients_lock.insert(player_id, ConnectedClient {
                player_id,
                addr,
                player_name: player_name.clone(),
                last_heartbeat: Instant::now(),
                tx: tx.clone(),
            });
        }

        // Notify game of new client
        let _ = event_tx.send(HostEvent::ClientConnected {
            player_id,
            player_name: player_name.clone(),
        });

        // Split stream for concurrent read/write
        let (mut read_half, mut write_half) = stream.into_split();

        // Spawn write task
        let write_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = NetworkManager::send_message_async(&mut write_half, &message).await {
                    eprintln!("Send error: {}", e);
                    break;
                }
            }
        });

        // Read loop
        loop {
            let message = match NetworkManager::receive_message_async(&mut read_half).await {
                Ok(msg) => msg,
                Err(_) => break, // Connection closed
            };

            // Update last heartbeat
            if let Ok(mut clients_lock) = clients.lock() {
                if let Some(client) = clients_lock.get_mut(&player_id) {
                    client.last_heartbeat = Instant::now();
                }
            }

            // Handle message
            match message {
                NetworkMessage::Heartbeat => {
                    // Heartbeat received, already updated timestamp above
                }
                NetworkMessage::Disconnect { .. } => {
                    break; // Client disconnecting
                }
                NetworkMessage::PlayerInput { player_id: _, thrust, rotate_left, rotate_right, launch, convert_to_satellite } => {
                    let _ = event_tx.send(HostEvent::PlayerInput {
                        player_id,
                        thrust,
                        rotate_left,
                        rotate_right,
                        launch,
                        convert_to_satellite,
                    });
                }
                _ => {
                    // Ignore other message types
                }
            }
        }

        // Clean up client
        write_task.abort();

        {
            let mut clients_lock = clients.lock().unwrap();
            clients_lock.remove(&player_id);
        }

        let _ = event_tx.send(HostEvent::ClientDisconnected { player_id });

        println!("Player {} ({}) disconnected", player_id, player_name);

        Ok(())
    }

    /// Monitor client heartbeats and disconnect timed-out clients (async task)
    async fn run_heartbeat_monitor(clients: Arc<Mutex<HashMap<u32, ConnectedClient>>>) {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);

        loop {
            interval.tick().await;

            let mut timed_out_clients = Vec::new();

            // Check for timed-out clients
            if let Ok(clients_lock) = clients.lock() {
                let now = Instant::now();

                for (player_id, client) in clients_lock.iter() {
                    if now.duration_since(client.last_heartbeat) > CLIENT_TIMEOUT {
                        timed_out_clients.push(*player_id);
                    }
                }
            }

            // Disconnect timed-out clients
            if !timed_out_clients.is_empty() {
                if let Ok(mut clients_lock) = clients.lock() {
                    for player_id in timed_out_clients {
                        if let Some(client) = clients_lock.get(&player_id) {
                            println!("Player {} timed out", player_id);
                            let _ = client.tx.send(NetworkMessage::Disconnect {
                                reason: "Connection timeout".to_string(),
                            });
                        }
                        clients_lock.remove(&player_id);
                    }
                }
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }
}

impl Default for MultiplayerHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MultiplayerHost {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_creation() {
        let host = MultiplayerHost::new();
        assert!(!host.is_running());
        assert_eq!(host.client_count(), 0);
    }

    #[test]
    fn test_host_state() {
        let mut host = MultiplayerHost::new();
        assert_eq!(host.current_frame(), 0);

        host.current_frame = 100;
        assert_eq!(host.current_frame(), 100);
    }

    #[test]
    fn test_stop_when_not_running() {
        let mut host = MultiplayerHost::new();
        host.stop(); // Should not panic
        assert!(!host.is_running());
    }
}
