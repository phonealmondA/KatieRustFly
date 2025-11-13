// Menu screens module

pub mod main_menu;
pub mod saves_menu;
pub mod map_selection_menu;
pub mod multiplayer_menu;
pub mod online_multiplayer_menu;
pub mod online_host_menu;
pub mod multiplayer_saves_menu;
pub mod online_join_menu;

pub use main_menu::MainMenu;
pub use saves_menu::{SavesMenu, SavesMenuResult};
pub use map_selection_menu::{MapSelectionMenu, MapSelectionResult};
pub use multiplayer_menu::{MultiplayerMenu, MultiplayerMenuResult};
pub use online_multiplayer_menu::{OnlineMultiplayerMenu, OnlineMultiplayerMenuResult};
pub use online_host_menu::{OnlineHostMenu, OnlineHostMenuResult};
pub use multiplayer_saves_menu::{MultiplayerSavesMenu, MultiplayerSavesMenuResult};
pub use online_join_menu::{OnlineJoinMenu, OnlineJoinMenuResult};
