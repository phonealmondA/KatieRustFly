// Player - Comprehensive player management for single and multiplayer
// Handles player state, input, vehicle management, and network synchronization

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::Rocket;
use crate::systems::{VehicleManager, SatelliteManager, EntityId};

/// Player type - local or remote
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerType {
    Local,
    Remote,
}

/// Serializable Vec2 wrapper
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SerializableVec2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for SerializableVec2 {
    fn from(v: Vec2) -> Self {
        SerializableVec2 { x: v.x, y: v.y }
    }
}

impl From<SerializableVec2> for Vec2 {
    fn from(v: SerializableVec2) -> Self {
        Vec2::new(v.x, v.y)
    }
}

/// Player state for network synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub player_id: usize,
    pub position: SerializableVec2,
    pub velocity: SerializableVec2,
    pub rotation: f32,
    pub thrust_level: f32,
    pub fuel: f32,
    pub mass: f32,
    pub is_thrusting: bool,
    pub timestamp: f32,
}

/// Input state tracking
#[derive(Debug, Clone, Default)]
struct InputState {
    // Fuel transfer keys
    fuel_increase_pressed: bool,
    fuel_decrease_pressed: bool,

    // Satellite conversion
    satellite_conversion_pressed: bool,

    // Launch/detach
    launch_pressed: bool,

    // Camera toggle
    camera_toggle_pressed: bool,
}

/// Player - Manages player state and interactions
pub struct Player {
    // Core identity
    player_id: usize,
    player_name: String,
    player_type: PlayerType,

    // Spawn management
    spawn_position: Vec2,

    // Vehicle management
    vehicle_manager: VehicleManager,
    active_rocket_id: Option<EntityId>,

    // Input tracking
    input_state: InputState,
    selected_thrust_level: f32,

    // Network synchronization
    state_changed: bool,
    time_since_last_state_sent: f32,
    state_send_interval: f32, // 30 FPS = 0.033s

    // External references
    nearby_planet_ids: Vec<EntityId>,
}

impl Player {
    /// Create a new local player
    pub fn new_local(player_id: usize, player_name: String, spawn_position: Vec2) -> Self {
        Player {
            player_id,
            player_name,
            player_type: PlayerType::Local,
            spawn_position,
            vehicle_manager: VehicleManager::new(),
            active_rocket_id: None,
            input_state: InputState::default(),
            selected_thrust_level: 1.0,
            state_changed: false,
            time_since_last_state_sent: 0.0,
            state_send_interval: 1.0 / 30.0, // 30 FPS
            nearby_planet_ids: Vec::new(),
        }
    }

    /// Create a new remote player
    pub fn new_remote(player_id: usize, player_name: String) -> Self {
        Player {
            player_id,
            player_name,
            player_type: PlayerType::Remote,
            spawn_position: Vec2::ZERO,
            vehicle_manager: VehicleManager::new(),
            active_rocket_id: None,
            input_state: InputState::default(),
            selected_thrust_level: 1.0,
            state_changed: false,
            time_since_last_state_sent: 0.0,
            state_send_interval: 1.0 / 30.0,
            nearby_planet_ids: Vec::new(),
        }
    }

    // === Getters ===

    pub fn player_id(&self) -> usize {
        self.player_id
    }

    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    pub fn player_type(&self) -> PlayerType {
        self.player_type
    }

    pub fn spawn_position(&self) -> Vec2 {
        self.spawn_position
    }

    pub fn set_spawn_position(&mut self, position: Vec2) {
        self.spawn_position = position;
    }

    pub fn active_rocket_id(&self) -> Option<EntityId> {
        self.active_rocket_id
    }

    pub fn set_active_rocket_id(&mut self, rocket_id: Option<EntityId>) {
        self.active_rocket_id = rocket_id;
        self.state_changed = true;
    }

    pub fn selected_thrust_level(&self) -> f32 {
        self.selected_thrust_level
    }

    pub fn vehicle_manager(&self) -> &VehicleManager {
        &self.vehicle_manager
    }

    pub fn vehicle_manager_mut(&mut self) -> &mut VehicleManager {
        &mut self.vehicle_manager
    }

    // === Planet Management ===

    pub fn set_nearby_planets(&mut self, planet_ids: Vec<EntityId>) {
        self.nearby_planet_ids = planet_ids;
    }

    pub fn nearby_planets(&self) -> &[EntityId] {
        &self.nearby_planet_ids
    }

    // === Input Handling ===

    /// Handle local player input (for local players only)
    pub fn handle_local_input(&mut self, active_rocket: Option<&mut Rocket>) {
        if self.player_type != PlayerType::Local {
            return;
        }

        if let Some(rocket) = active_rocket {
            // Thrust control
            if is_key_down(KeyCode::Space) {
                rocket.set_thrust_level(self.selected_thrust_level);
                self.state_changed = true;
            } else {
                rocket.set_thrust_level(0.0);
            }

            // Rotation
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                rocket.rotate(-3.0);
                self.state_changed = true;
            }

            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                rocket.rotate(3.0);
                self.state_changed = true;
            }
        }

        // Fuel transfer input
        self.handle_fuel_transfer_input();

        // Satellite conversion input
        self.handle_satellite_conversion_input();

        // Launch input
        self.handle_launch_input();

        // Camera toggle
        self.handle_camera_toggle_input();
    }

    /// Handle fuel transfer input (thrust level adjustment)
    fn handle_fuel_transfer_input(&mut self) {
        // Increase thrust level with '.' key
        if is_key_down(KeyCode::Period) {
            if !self.input_state.fuel_increase_pressed {
                self.selected_thrust_level = (self.selected_thrust_level + 0.1).min(1.0);
                self.input_state.fuel_increase_pressed = true;
                self.state_changed = true;
            }
        } else {
            self.input_state.fuel_increase_pressed = false;
        }

        // Decrease thrust level with ',' key
        if is_key_down(KeyCode::Comma) {
            if !self.input_state.fuel_decrease_pressed {
                self.selected_thrust_level = (self.selected_thrust_level - 0.1).max(0.0);
                self.input_state.fuel_decrease_pressed = true;
                self.state_changed = true;
            }
        } else {
            self.input_state.fuel_decrease_pressed = false;
        }
    }

    /// Handle satellite conversion input
    fn handle_satellite_conversion_input(&mut self) {
        if is_key_pressed(KeyCode::C) {
            if !self.input_state.satellite_conversion_pressed {
                // Request satellite conversion
                // This will be handled by the game mode
                self.input_state.satellite_conversion_pressed = true;
                self.state_changed = true;
            }
        } else {
            self.input_state.satellite_conversion_pressed = false;
        }
    }

    /// Check if player wants to convert to satellite
    pub fn wants_satellite_conversion(&self) -> bool {
        self.input_state.satellite_conversion_pressed
    }

    /// Clear satellite conversion flag
    pub fn clear_satellite_conversion_flag(&mut self) {
        self.input_state.satellite_conversion_pressed = false;
    }

    /// Handle launch/detach input
    fn handle_launch_input(&mut self) {
        if is_key_pressed(KeyCode::E) {
            if !self.input_state.launch_pressed {
                // Request launch/detach
                self.input_state.launch_pressed = true;
                self.state_changed = true;
            }
        } else {
            self.input_state.launch_pressed = false;
        }
    }

    /// Check if player wants to launch
    pub fn wants_launch(&self) -> bool {
        self.input_state.launch_pressed
    }

    /// Clear launch flag
    pub fn clear_launch_flag(&mut self) {
        self.input_state.launch_pressed = false;
    }

    /// Handle camera toggle input
    fn handle_camera_toggle_input(&mut self) {
        if is_key_pressed(KeyCode::F) {
            if !self.input_state.camera_toggle_pressed {
                // Request camera toggle
                self.input_state.camera_toggle_pressed = true;
            }
        } else {
            self.input_state.camera_toggle_pressed = false;
        }
    }

    /// Check if player wants to toggle camera
    pub fn wants_camera_toggle(&self) -> bool {
        self.input_state.camera_toggle_pressed
    }

    /// Clear camera toggle flag
    pub fn clear_camera_toggle_flag(&mut self) {
        self.input_state.camera_toggle_pressed = false;
    }

    // === Satellite System ===

    /// Check if player can convert rocket to satellite
    pub fn can_convert_to_satellite(
        &self,
        active_rocket: &Rocket,
        satellite_manager: &SatelliteManager,
    ) -> bool {
        satellite_manager.can_convert_rocket_to_satellite(
            active_rocket,
            200.0, // Min altitude
            50.0,  // Min fuel
        )
    }

    /// Convert active rocket to satellite
    pub fn convert_rocket_to_satellite(
        &mut self,
        active_rocket: &Rocket,
        satellite_manager: &mut SatelliteManager,
    ) -> Option<EntityId> {
        let satellite_id = satellite_manager.create_satellite_from_rocket(
            active_rocket.position(),
            active_rocket.velocity(),
            active_rocket.current_fuel(),
            Some(self.player_id),
        );

        // Clear active rocket
        self.active_rocket_id = None;
        self.state_changed = true;

        Some(satellite_id)
    }

    // === Fuel Management ===

    pub fn get_current_fuel(&self, active_rocket: Option<&Rocket>) -> f32 {
        active_rocket.map(|r| r.current_fuel()).unwrap_or(0.0)
    }

    pub fn get_max_fuel(&self, active_rocket: Option<&Rocket>) -> f32 {
        active_rocket.map(|r| r.max_fuel()).unwrap_or(0.0)
    }

    pub fn get_fuel_percentage(&self, active_rocket: Option<&Rocket>) -> f32 {
        active_rocket.map(|r| r.fuel_percentage()).unwrap_or(0.0)
    }

    pub fn can_thrust(&self, active_rocket: Option<&Rocket>) -> bool {
        self.get_current_fuel(active_rocket) > 0.0
    }

    // === Network Synchronization ===

    /// Get current player state for network transmission
    pub fn get_state(&self, active_rocket: Option<&Rocket>, game_time: f32) -> PlayerState {
        if let Some(rocket) = active_rocket {
            PlayerState {
                player_id: self.player_id,
                position: rocket.position().into(),
                velocity: rocket.velocity().into(),
                rotation: rocket.rotation(),
                thrust_level: rocket.thrust_level(),
                fuel: rocket.current_fuel(),
                mass: rocket.mass(),
                is_thrusting: rocket.thrust_level() > 0.0,
                timestamp: game_time,
            }
        } else {
            PlayerState {
                player_id: self.player_id,
                position: self.spawn_position.into(),
                velocity: Vec2::ZERO.into(),
                rotation: 0.0,
                thrust_level: 0.0,
                fuel: 0.0,
                mass: 0.0,
                is_thrusting: false,
                timestamp: game_time,
            }
        }
    }

    /// Apply state received from network (for remote players)
    pub fn apply_state(&mut self, state: PlayerState, active_rocket: Option<&mut Rocket>) {
        if self.player_type != PlayerType::Remote {
            return;
        }

        if let Some(rocket) = active_rocket {
            rocket.set_position(state.position.into());
            rocket.set_velocity(state.velocity.into());
            // Note: Rocket doesn't have set_rotation, it's managed through rotate() method
            rocket.set_thrust_level(state.thrust_level);
        }
    }

    /// Check if we should send state update over network
    pub fn should_send_state(&self, delta_time: f32) -> bool {
        if self.player_type != PlayerType::Local {
            return false;
        }

        self.state_changed || self.time_since_last_state_sent >= self.state_send_interval
    }

    /// Mark state as sent
    pub fn mark_state_sent(&mut self) {
        self.state_changed = false;
        self.time_since_last_state_sent = 0.0;
    }

    /// Update network timing
    pub fn update(&mut self, delta_time: f32) {
        self.time_since_last_state_sent += delta_time;
    }

    // === Respawn ===

    /// Respawn player at a specific position
    pub fn respawn_at_position(&mut self, position: Vec2) {
        self.spawn_position = position;
        self.active_rocket_id = None;
        self.state_changed = true;
    }

    /// Request vehicle transformation
    pub fn request_transform(&mut self, _new_vehicle_type: &str) {
        // Placeholder for future vehicle type switching (rocket/drone)
        self.state_changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_local_player() {
        let player = Player::new_local(1, "Player1".to_string(), Vec2::new(100.0, 100.0));
        assert_eq!(player.player_id(), 1);
        assert_eq!(player.player_name(), "Player1");
        assert_eq!(player.player_type(), PlayerType::Local);
        assert_eq!(player.spawn_position(), Vec2::new(100.0, 100.0));
    }

    #[test]
    fn test_create_remote_player() {
        let player = Player::new_remote(2, "Player2".to_string());
        assert_eq!(player.player_id(), 2);
        assert_eq!(player.player_name(), "Player2");
        assert_eq!(player.player_type(), PlayerType::Remote);
    }

    #[test]
    fn test_thrust_level_adjustment() {
        let mut player = Player::new_local(1, "Test".to_string(), Vec2::ZERO);

        assert_eq!(player.selected_thrust_level(), 1.0);

        // Simulate increasing thrust
        player.selected_thrust_level = (player.selected_thrust_level + 0.1).min(1.0);
        assert_eq!(player.selected_thrust_level(), 1.0); // Already at max

        // Simulate decreasing thrust
        player.selected_thrust_level = 0.5;
        assert_eq!(player.selected_thrust_level(), 0.5);
    }

    #[test]
    fn test_state_synchronization() {
        let player = Player::new_local(1, "Test".to_string(), Vec2::new(50.0, 50.0));

        let state = player.get_state(None, 1.0);
        assert_eq!(state.player_id, 1);
        assert_eq!(state.position, SerializableVec2 { x: 50.0, y: 50.0 });
        assert_eq!(state.timestamp, 1.0);
    }

    #[test]
    fn test_network_state_timing() {
        let mut player = Player::new_local(1, "Test".to_string(), Vec2::ZERO);

        // Should send immediately if state changed
        player.state_changed = true;
        assert!(player.should_send_state(0.01));

        player.mark_state_sent();
        assert!(!player.should_send_state(0.01));

        // Should send after interval
        player.update(0.034); // Slightly more than 30 FPS interval
        assert!(player.should_send_state(0.0));
    }

    #[test]
    fn test_spawn_position_management() {
        let mut player = Player::new_local(1, "Test".to_string(), Vec2::new(100.0, 100.0));

        player.respawn_at_position(Vec2::new(200.0, 200.0));
        assert_eq!(player.spawn_position(), Vec2::new(200.0, 200.0));
        assert!(player.state_changed);
    }
}
