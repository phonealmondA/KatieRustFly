// Networking module
// Phases 10-12: Networking infrastructure (placeholder implementations)

pub mod network_manager;
pub mod multiplayer_host;
pub mod multiplayer_client;

pub use network_manager::{NetworkManager, NetworkRole, NetworkMessage};
pub use multiplayer_host::MultiplayerHost;
pub use multiplayer_client::MultiplayerClient;
