// Networking module
// Phases 10-12: Networking infrastructure with tokio async networking

pub mod network_manager;
pub mod multiplayer_host;
pub mod multiplayer_client;

pub use network_manager::{
    NetworkManager, NetworkRole, NetworkMessage, NetworkError,
    NetworkStats, PlayerStateData, GameStateSnapshot,
};
pub use multiplayer_host::{MultiplayerHost, HostEvent};
pub use multiplayer_client::MultiplayerClient;
