// Multiplayer Client - Client-side prediction and interpolation
// Phase 12: Multiplayer Client

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::networking::{
    NetworkManager, NetworkMessage, NetworkError,
    PlayerStateData, GameStateSnapshot,
};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const SERVER_TIMEOUT: Duration = Duration::from_secs(15);
const INTERPOLATION_DELAY: f32 = 0.1; // 100ms interpolation buffer

/// Remote player state for interpolation
#[derive(Debug, Clone)]
struct RemotePlayerState {
    player_id: u32,
    position: (f32, f32),
    velocity: (f32, f32),
    rotation: f32,
    fuel: f32,
    is_alive: bool,
    timestamp: f64,
}

/// Client events for game integration
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected { player_id: u32 },
    Disconnected { reason: String },
    GameStateReceived { frame: u64, timestamp: f64 },
    PlayerJoined { player_id: u32 },
    PlayerLeft { player_id: u32 },
}

/// Multiplayer Client with client-side prediction and interpolation
pub struct MultiplayerClient {
    connected: bool,
    player_id: Option<u32>,
    server_addr: Option<SocketAddr>,

    // Remote player states
    remote_players: Arc<Mutex<HashMap<u32, RemotePlayerState>>>,

    // Previous states for interpolation
    previous_states: HashMap<u32, RemotePlayerState>,

    // Server state
    last_server_update: Instant,
    current_server_frame: u64,
    server_time: f64,

    // Event channels
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<ClientEvent>>,

    // Message channels
    message_tx: Option<mpsc::UnboundedSender<NetworkMessage>>,

    // Background tasks
    connection_task: Option<JoinHandle<()>>,
    heartbeat_task: Option<JoinHandle<()>>,
}

impl MultiplayerClient {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        MultiplayerClient {
            connected: false,
            player_id: None,
            server_addr: None,
            remote_players: Arc::new(Mutex::new(HashMap::new())),
            previous_states: HashMap::new(),
            last_server_update: Instant::now(),
            current_server_frame: 0,
            server_time: 0.0,
            event_tx,
            event_rx: Some(event_rx),
            message_tx: None,
            connection_task: None,
            heartbeat_task: None,
        }
    }

    /// Connect to host
    pub fn connect(&mut self, address: &str) -> Result<(), String> {
        if self.connected {
            return Err("Already connected".to_string());
        }

        let addr: SocketAddr = address.parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        self.server_addr = Some(addr);

        // Spawn connection task
        let remote_players = Arc::clone(&self.remote_players);
        let event_tx = self.event_tx.clone();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        self.message_tx = Some(msg_tx.clone());

        let connection_task = tokio::spawn(async move {
            if let Err(e) = Self::run_connection(addr, remote_players, event_tx.clone(), msg_tx, msg_rx).await {
                eprintln!("Connection error: {}", e);
                let _ = event_tx.send(ClientEvent::Disconnected {
                    reason: e.to_string(),
                });
            }
        });

        self.connection_task = Some(connection_task);

        // Spawn heartbeat task
        let message_tx = self.message_tx.clone();
        let heartbeat_task = tokio::spawn(async move {
            if let Some(tx) = message_tx {
                Self::run_heartbeat(tx).await;
            }
        });

        self.heartbeat_task = Some(heartbeat_task);

        Ok(())
    }

    /// Disconnect from server
    pub fn disconnect(&mut self) {
        if !self.connected {
            return;
        }

        // Send disconnect message
        if let Some(tx) = &self.message_tx {
            let _ = tx.send(NetworkMessage::Disconnect {
                reason: "Client disconnecting".to_string(),
            });
        }

        self.connected = false;
        self.player_id = None;
        self.server_addr = None;

        // Clear remote players
        if let Ok(mut players) = self.remote_players.lock() {
            players.clear();
        }

        // Abort background tasks
        if let Some(task) = self.connection_task.take() {
            task.abort();
        }
        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
    }

    /// Update client state (interpolation)
    pub fn update(&mut self, delta_time: f32) {
        if !self.connected {
            return;
        }

        // Check for server timeout
        if self.last_server_update.elapsed() > SERVER_TIMEOUT {
            let _ = self.event_tx.send(ClientEvent::Disconnected {
                reason: "Server timeout".to_string(),
            });
            self.disconnect();
            return;
        }

        // Update interpolation for remote players
        self.interpolate_remote_players(delta_time);
    }

    /// Interpolate remote player states
    fn interpolate_remote_players(&mut self, delta_time: f32) {
        let target_time = self.server_time - INTERPOLATION_DELAY as f64;

        if let Ok(mut players) = self.remote_players.lock() {
            for (player_id, current_state) in players.iter_mut() {
                if let Some(previous_state) = self.previous_states.get(player_id) {
                    // Calculate interpolation factor
                    let time_diff = current_state.timestamp - previous_state.timestamp;
                    if time_diff > 0.0 {
                        let elapsed = target_time - previous_state.timestamp;
                        let t = (elapsed / time_diff).clamp(0.0, 1.0) as f32;

                        // Interpolate position
                        current_state.position = (
                            previous_state.position.0 + (current_state.position.0 - previous_state.position.0) * t,
                            previous_state.position.1 + (current_state.position.1 - previous_state.position.1) * t,
                        );

                        // Interpolate velocity
                        current_state.velocity = (
                            previous_state.velocity.0 + (current_state.velocity.0 - previous_state.velocity.0) * t,
                            previous_state.velocity.1 + (current_state.velocity.1 - previous_state.velocity.1) * t,
                        );

                        // Interpolate rotation
                        current_state.rotation = previous_state.rotation + (current_state.rotation - previous_state.rotation) * t;
                    }
                }
            }
        }
    }

    /// Send player input to server
    pub fn send_input(&mut self, thrust: bool, rotate_left: bool, rotate_right: bool, launch: bool, convert_to_satellite: bool) {
        if !self.connected {
            return;
        }

        if let Some(player_id) = self.player_id {
            if let Some(tx) = &self.message_tx {
                let _ = tx.send(NetworkMessage::PlayerInput {
                    player_id,
                    thrust,
                    rotate_left,
                    rotate_right,
                    launch,
                    convert_to_satellite,
                });
            }
        }
    }

    /// Get remote player state
    pub fn get_remote_player(&self, player_id: u32) -> Option<PlayerStateData> {
        if let Ok(players) = self.remote_players.lock() {
            players.get(&player_id).map(|state| PlayerStateData {
                player_id: state.player_id,
                position: state.position,
                velocity: state.velocity,
                rotation: state.rotation,
                fuel: state.fuel,
                is_alive: state.is_alive,
            })
        } else {
            None
        }
    }

    /// Get all remote players
    pub fn get_all_remote_players(&self) -> Vec<PlayerStateData> {
        if let Ok(players) = self.remote_players.lock() {
            players.values().map(|state| PlayerStateData {
                player_id: state.player_id,
                position: state.position,
                velocity: state.velocity,
                rotation: state.rotation,
                fuel: state.fuel,
                is_alive: state.is_alive,
            }).collect()
        } else {
            Vec::new()
        }
    }

    /// Poll client events
    pub fn poll_events(&mut self) -> Vec<ClientEvent> {
        let mut events = Vec::new();

        if let Some(rx) = &mut self.event_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        events
    }

    /// Run connection (async task)
    async fn run_connection(
        addr: SocketAddr,
        remote_players: Arc<Mutex<HashMap<u32, RemotePlayerState>>>,
        event_tx: mpsc::UnboundedSender<ClientEvent>,
        message_tx: mpsc::UnboundedSender<NetworkMessage>,
        mut message_rx: mpsc::UnboundedReceiver<NetworkMessage>,
    ) -> Result<(), NetworkError> {
        // Connect to server
        let mut stream = TcpStream::connect(addr).await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        println!("Connected to server at {}", addr);

        // Send handshake
        let handshake = NetworkMessage::Handshake {
            version: 1,
            player_name: "Player".to_string(), // TODO: Get from configuration
        };

        NetworkManager::send_message_async(&mut stream, &handshake).await?;

        // Receive handshake acknowledgment
        let ack = NetworkManager::receive_message_async(&mut stream).await?;

        let player_id = match ack {
            NetworkMessage::HandshakeAck { player_id, game_state: _ } => player_id,
            _ => return Err(NetworkError::ConnectionFailed("Expected handshake ack".to_string())),
        };

        println!("Assigned player ID: {}", player_id);

        // Notify connected
        let _ = event_tx.send(ClientEvent::Connected { player_id });

        // Split stream for concurrent read/write
        let (mut read_half, mut write_half) = stream.into_split();

        // Spawn write task
        let write_task = tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
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

            // Handle message
            match message {
                NetworkMessage::GameStateUpdate { frame, timestamp, players } => {
                    // Update remote players
                    if let Ok(mut remote_players_lock) = remote_players.lock() {
                        for player_state in players {
                            // Skip self
                            if player_state.player_id == player_id {
                                continue;
                            }

                            // Check if this is a new player
                            let is_new = !remote_players_lock.contains_key(&player_state.player_id);

                            remote_players_lock.insert(player_state.player_id, RemotePlayerState {
                                player_id: player_state.player_id,
                                position: player_state.position,
                                velocity: player_state.velocity,
                                rotation: player_state.rotation,
                                fuel: player_state.fuel,
                                is_alive: player_state.is_alive,
                                timestamp,
                            });

                            if is_new {
                                let _ = event_tx.send(ClientEvent::PlayerJoined {
                                    player_id: player_state.player_id,
                                });
                            }
                        }
                    }

                    let _ = event_tx.send(ClientEvent::GameStateReceived { frame, timestamp });
                }
                NetworkMessage::PlayerDisconnect { player_id: disconnected_id } => {
                    if let Ok(mut players) = remote_players.lock() {
                        players.remove(&disconnected_id);
                    }

                    let _ = event_tx.send(ClientEvent::PlayerLeft {
                        player_id: disconnected_id,
                    });
                }
                NetworkMessage::Disconnect { reason } => {
                    let _ = event_tx.send(ClientEvent::Disconnected { reason });
                    break;
                }
                _ => {
                    // Ignore other message types
                }
            }
        }

        // Clean up
        write_task.abort();

        Ok(())
    }

    /// Run heartbeat sender (async task)
    async fn run_heartbeat(message_tx: mpsc::UnboundedSender<NetworkMessage>) {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);

        loop {
            interval.tick().await;

            if message_tx.send(NetworkMessage::Heartbeat).is_err() {
                break; // Channel closed
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }

    pub fn server_addr(&self) -> Option<SocketAddr> {
        self.server_addr
    }

    pub fn current_frame(&self) -> u64 {
        self.current_server_frame
    }

    pub fn remote_player_count(&self) -> usize {
        self.remote_players.lock().map(|p| p.len()).unwrap_or(0)
    }
}

impl Default for MultiplayerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MultiplayerClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MultiplayerClient::new();
        assert!(!client.is_connected());
        assert_eq!(client.player_id(), None);
    }

    #[test]
    fn test_client_state() {
        let client = MultiplayerClient::new();
        assert_eq!(client.current_frame(), 0);
        assert_eq!(client.remote_player_count(), 0);
    }

    #[test]
    fn test_disconnect_when_not_connected() {
        let mut client = MultiplayerClient::new();
        client.disconnect(); // Should not panic
        assert!(!client.is_connected());
    }

    #[test]
    fn test_get_remote_players_empty() {
        let client = MultiplayerClient::new();
        let players = client.get_all_remote_players();
        assert_eq!(players.len(), 0);
    }
}
