// Player Input System - Input abstraction for multiplayer support
// Allows multiple players with different key bindings

use macroquad::prelude::*;

/// Input configuration for a single player
#[derive(Debug, Clone)]
pub struct PlayerInput {
    pub player_id: u32,

    // Movement controls
    pub rotate_left: KeyCode,
    pub rotate_right: KeyCode,
    pub thrust: KeyCode,

    // Thrust level adjustment
    pub decrease_thrust: KeyCode,
    pub increase_thrust: KeyCode,

    // Camera zoom
    pub zoom_out: KeyCode,
    pub zoom_in: KeyCode,

    // Actions
    pub convert_to_satellite: KeyCode,
    pub camera_focus: KeyCode,  // Focus camera on this player
}

impl PlayerInput {
    /// Create Player 1 input configuration (original single-player controls)
    pub fn player1() -> Self {
        PlayerInput {
            player_id: 0,
            rotate_left: KeyCode::D,
            rotate_right: KeyCode::A,
            thrust: KeyCode::W,
            decrease_thrust: KeyCode::Z,
            increase_thrust: KeyCode::X,
            zoom_out: KeyCode::Q,
            zoom_in: KeyCode::E,
            convert_to_satellite: KeyCode::C,
            camera_focus: KeyCode::R,
        }
    }

    /// Create Player 2 input configuration (split-screen controls)
    pub fn player2() -> Self {
        PlayerInput {
            player_id: 1,
            rotate_left: KeyCode::Right,
            rotate_right: KeyCode::Left,
            thrust: KeyCode::Up,
            decrease_thrust: KeyCode::Comma,
            increase_thrust: KeyCode::Period,
            zoom_out: KeyCode::Slash,
            zoom_in: KeyCode::Apostrophe,
            convert_to_satellite: KeyCode::RightBracket,
            camera_focus: KeyCode::Semicolon,
        }
    }

    /// Get the rotation input for this frame (-1.0 = left, 1.0 = right, 0.0 = none)
    pub fn get_rotation_input(&self) -> f32 {
        let mut rotation = 0.0;

        if is_key_down(self.rotate_left) {
            rotation -= 1.0;
        }
        if is_key_down(self.rotate_right) {
            rotation += 1.0;
        }

        rotation
    }

    /// Check if thrust key is pressed
    pub fn is_thrusting(&self) -> bool {
        is_key_down(self.thrust)
    }

    /// Check if decrease thrust was just pressed
    pub fn just_decreased_thrust(&self) -> bool {
        is_key_pressed(self.decrease_thrust)
    }

    /// Check if increase thrust was just pressed
    pub fn just_increased_thrust(&self) -> bool {
        is_key_pressed(self.increase_thrust)
    }

    /// Check if zoom out key is down
    pub fn is_zooming_out(&self) -> bool {
        is_key_down(self.zoom_out)
    }

    /// Check if zoom in key is down
    pub fn is_zooming_in(&self) -> bool {
        is_key_down(self.zoom_in)
    }

    /// Check if convert to satellite was just pressed
    pub fn just_converted_to_satellite(&self) -> bool {
        is_key_pressed(self.convert_to_satellite)
    }

    /// Check if camera focus was just pressed
    pub fn just_focused_camera(&self) -> bool {
        is_key_pressed(self.camera_focus)
    }
}

/// Per-player state that needs to be tracked
#[derive(Debug, Clone)]
pub struct PlayerInputState {
    pub player_id: u32,
    pub selected_thrust_level: f32,  // 0.0 to 1.0
}

impl PlayerInputState {
    pub fn new(player_id: u32) -> Self {
        PlayerInputState {
            player_id,
            selected_thrust_level: 0.0,
        }
    }

    /// Adjust thrust level by delta (-0.05 or +0.05)
    pub fn adjust_thrust(&mut self, delta: f32) {
        self.selected_thrust_level = (self.selected_thrust_level + delta).clamp(0.0, 1.0);
    }

    /// Get current thrust level
    pub fn thrust_level(&self) -> f32 {
        self.selected_thrust_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_input_creation() {
        let p1 = PlayerInput::player1();
        assert_eq!(p1.player_id, 0);
        assert_eq!(p1.rotate_left, KeyCode::A);

        let p2 = PlayerInput::player2();
        assert_eq!(p2.player_id, 1);
        assert_eq!(p2.rotate_left, KeyCode::Left);
    }

    #[test]
    fn test_player_input_state() {
        let mut state = PlayerInputState::new(0);
        assert_eq!(state.thrust_level(), 0.0);

        state.adjust_thrust(0.05);
        assert_eq!(state.thrust_level(), 0.05);

        state.adjust_thrust(-0.1);
        assert_eq!(state.thrust_level(), 0.0); // Clamped to 0.0
    }
}
