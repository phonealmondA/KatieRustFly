// Game modes module

pub mod single_player;
pub mod split_screen;
pub mod multiplayer_host;
pub mod multiplayer_client;

pub use single_player::{SinglePlayerGame, SinglePlayerResult};
pub use split_screen::{SplitScreenGame, SplitScreenResult};
pub use multiplayer_host::{MultiplayerHost, MultiplayerHostResult};
pub use multiplayer_client::{MultiplayerClient, MultiplayerClientResult};
