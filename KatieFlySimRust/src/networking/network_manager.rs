// Network Manager - Async networking infrastructure with tokio
// Phase 10: Networking Foundation

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

/// Network role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkRole {
    Host,
    Client,
    None,
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // Connection management
    Handshake {
        version: u32,
        player_name: String,
    },
    HandshakeAck {
        player_id: u32,
        game_state: GameStateSnapshot,
    },
    Heartbeat,
    Disconnect {
        reason: String,
    },

    // Game state
    PlayerState {
        player_id: u32,
        position: (f32, f32),
        velocity: (f32, f32),
        rotation: f32,
        fuel: f32,
        thrust_level: f32,
    },
    PlayerInput {
        player_id: u32,
        thrust: bool,
        rotate_left: bool,
        rotate_right: bool,
        launch: bool,
        convert_to_satellite: bool,
    },
    PlayerSpawn {
        player_id: u32,
        position: (f32, f32),
        velocity: (f32, f32),
    },
    PlayerDisconnect {
        player_id: u32,
    },

    // World state
    GameStateUpdate {
        frame: u64,
        timestamp: f64,
        players: Vec<PlayerStateData>,
    },
    SatelliteConversion {
        rocket_id: u32,
        satellite_id: u32,
    },
}

/// Player state data for networking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStateData {
    pub player_id: u32,
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub rotation: f32,
    pub fuel: f32,
    pub is_alive: bool,
}

/// Game state snapshot for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub frame: u64,
    pub players: Vec<PlayerStateData>,
}

/// Network error types
#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed(String),
    Disconnected,
    SerializationError(String),
    Timeout,
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            NetworkError::Disconnected => write!(f, "Disconnected"),
            NetworkError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            NetworkError::Timeout => write!(f, "Connection timeout"),
        }
    }
}

impl std::error::Error for NetworkError {}

/// Network Manager with tokio async networking
pub struct NetworkManager {
    role: NetworkRole,
    connected: bool,
    local_addr: Option<SocketAddr>,

    // Async communication channels
    message_tx: Option<mpsc::UnboundedSender<NetworkMessage>>,
    message_rx: Option<mpsc::UnboundedReceiver<NetworkMessage>>,

    // Statistics
    bytes_sent: Arc<Mutex<u64>>,
    bytes_received: Arc<Mutex<u64>>,
    messages_sent: Arc<Mutex<u64>>,
    messages_received: Arc<Mutex<u64>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        NetworkManager {
            role: NetworkRole::None,
            connected: false,
            local_addr: None,
            message_tx: Some(tx),
            message_rx: Some(rx),
            bytes_sent: Arc::new(Mutex::new(0)),
            bytes_received: Arc::new(Mutex::new(0)),
            messages_sent: Arc::new(Mutex::new(0)),
            messages_received: Arc::new(Mutex::new(0)),
        }
    }

    pub fn role(&self) -> NetworkRole {
        self.role
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    /// Serialize message to bytes
    fn serialize_message(message: &NetworkMessage) -> Result<Vec<u8>, NetworkError> {
        bincode::serialize(message)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// Deserialize message from bytes
    fn deserialize_message(bytes: &[u8]) -> Result<NetworkMessage, NetworkError> {
        bincode::deserialize(bytes)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// Send message through TCP stream
    async fn send_message_async(
        stream: &mut TcpStream,
        message: &NetworkMessage,
    ) -> Result<(), NetworkError> {
        let data = Self::serialize_message(message)?;
        let len = data.len() as u32;

        // Send length prefix (4 bytes)
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // Send message data
        stream.write_all(&data).await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        Ok(())
    }

    /// Receive message from TCP stream
    async fn receive_message_async(
        stream: &mut TcpStream,
    ) -> Result<NetworkMessage, NetworkError> {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await
            .map_err(|_| NetworkError::Disconnected)?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // Sanity check message size (max 10MB)
        if len > 10 * 1024 * 1024 {
            return Err(NetworkError::SerializationError("Message too large".to_string()));
        }

        // Read message data
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await
            .map_err(|_| NetworkError::Disconnected)?;

        Self::deserialize_message(&data)
    }

    /// Send message (queues for async sending)
    pub fn send_message(&mut self, message: NetworkMessage) -> Result<(), String> {
        if let Some(tx) = &self.message_tx {
            tx.send(message)
                .map_err(|e| format!("Failed to queue message: {}", e))?;

            let mut count = self.messages_sent.lock().unwrap();
            *count += 1;

            Ok(())
        } else {
            Err("Network not initialized".to_string())
        }
    }

    /// Receive queued messages
    pub fn receive_messages(&mut self) -> Vec<NetworkMessage> {
        let mut messages = Vec::new();

        if let Some(rx) = &mut self.message_rx {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);

                let mut count = self.messages_received.lock().unwrap();
                *count += 1;
            }
        }

        messages
    }

    /// Disconnect
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.role = NetworkRole::None;
        self.local_addr = None;
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            bytes_sent: *self.bytes_sent.lock().unwrap(),
            bytes_received: *self.bytes_received.lock().unwrap(),
            messages_sent: *self.messages_sent.lock().unwrap(),
            messages_received: *self.messages_received.lock().unwrap(),
        }
    }
}

/// Network statistics
#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_manager_creation() {
        let manager = NetworkManager::new();
        assert_eq!(manager.role(), NetworkRole::None);
        assert!(!manager.is_connected());
    }

    #[test]
    fn test_message_serialization() {
        let message = NetworkMessage::Heartbeat;
        let serialized = NetworkManager::serialize_message(&message).unwrap();
        let deserialized = NetworkManager::deserialize_message(&serialized).unwrap();

        matches!(deserialized, NetworkMessage::Heartbeat);
    }

    #[test]
    fn test_player_state_message() {
        let message = NetworkMessage::PlayerState {
            player_id: 1,
            position: (100.0, 200.0),
            velocity: (10.0, 20.0),
            rotation: 1.57,
            fuel: 50.0,
            thrust_level: 0.8,
        };

        let serialized = NetworkManager::serialize_message(&message).unwrap();
        assert!(!serialized.is_empty());

        let deserialized = NetworkManager::deserialize_message(&serialized).unwrap();
        match deserialized {
            NetworkMessage::PlayerState { player_id, .. } => {
                assert_eq!(player_id, 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_queue() {
        let mut manager = NetworkManager::new();

        let msg = NetworkMessage::Heartbeat;
        manager.send_message(msg).unwrap();

        let received = manager.receive_messages();
        assert_eq!(received.len(), 1);
    }

    #[test]
    fn test_network_stats() {
        let manager = NetworkManager::new();
        let stats = manager.get_stats();

        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }
}
