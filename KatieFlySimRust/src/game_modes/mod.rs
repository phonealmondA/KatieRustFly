// Game modes module

pub mod single_player;
pub mod split_screen;

pub use single_player::{SinglePlayerGame, SinglePlayerResult};
pub use split_screen::{SplitScreenMode, Viewport, PlayerInputMapping};
