// Satellite - Autonomous spacecraft for fuel collection
// Ported from C++ Satellite class (simplified)

use macroquad::prelude::*;

use super::game_object::{GameObject, GameObjectData};
use crate::game_constants::GameConstants;

/// Satellite for automated fuel collection and orbital maintenance
pub struct Satellite {
    data: GameObjectData,
    rotation: f32,

    // Mass and fuel
    mass: f32,
    base_mass: f32,
    max_mass: f32,
    current_fuel: f32,
    max_fuel: f32,
    maintenance_fuel_reserve: f32,

    // Orbital maintenance
    target_orbit_radius: f32,
    is_maintaining_orbit: bool,
    last_maintenance_time: f32,
    maintenance_interval: f32,

    // Fuel collection
    is_collecting_fuel: bool,
    fuel_source_planet_id: Option<usize>,
    collection_rate: f32,

    // Network & communication
    nearby_satellites: Vec<usize>,
    nearby_rockets: Vec<usize>,
    transfer_range: f32,
    is_transferring_fuel: bool,
}

impl Satellite {
    pub fn new(position: Vec2, velocity: Vec2, color: Color) -> Self {
        Satellite {
            data: GameObjectData::new(position, velocity, color),
            rotation: 0.0,
            mass: GameConstants::SATELLITE_BASE_MASS + GameConstants::SATELLITE_STARTING_FUEL,
            base_mass: GameConstants::SATELLITE_BASE_MASS,
            max_mass: GameConstants::SATELLITE_MAX_MASS,
            current_fuel: GameConstants::SATELLITE_STARTING_FUEL,
            max_fuel: GameConstants::SATELLITE_MAX_FUEL,
            maintenance_fuel_reserve: 20.0,
            target_orbit_radius: 0.0,
            is_maintaining_orbit: false,
            last_maintenance_time: 0.0,
            maintenance_interval: 5.0,
            is_collecting_fuel: false,
            fuel_source_planet_id: None,
            collection_rate: 1.0,
            nearby_satellites: Vec::new(),
            nearby_rockets: Vec::new(),
            transfer_range: 500.0,
            is_transferring_fuel: false,
        }
    }

    /// Create satellite from a rocket (conversion)
    pub fn from_rocket(position: Vec2, velocity: Vec2, rocket_fuel: f32) -> Self {
        let retained_fuel =
            rocket_fuel * GameConstants::SATELLITE_CONVERSION_FUEL_RETENTION;
        let fuel = retained_fuel.min(GameConstants::SATELLITE_MAX_FUEL);

        let mut satellite = Self::new(
            position,
            velocity,
            crate::game_constants::colors::SATELLITE_BODY_COLOR,
        );
        satellite.current_fuel = fuel;
        satellite.mass = satellite.base_mass + fuel;

        satellite
    }

    // === Getters ===
    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn current_fuel(&self) -> f32 {
        self.current_fuel
    }

    pub fn max_fuel(&self) -> f32 {
        self.max_fuel
    }

    pub fn fuel_percentage(&self) -> f32 {
        if self.max_fuel > 0.0 {
            (self.current_fuel / self.max_fuel) * 100.0
        } else {
            0.0
        }
    }

    pub fn is_maintaining_orbit(&self) -> bool {
        self.is_maintaining_orbit
    }

    pub fn set_maintaining_orbit(&mut self, maintaining: bool) {
        self.is_maintaining_orbit = maintaining;
    }

    pub fn target_orbit_radius(&self) -> f32 {
        self.target_orbit_radius
    }

    pub fn set_target_orbit_radius(&mut self, radius: f32) {
        self.target_orbit_radius = radius;
    }

    pub fn set_is_maintaining_orbit(&mut self, maintaining: bool) {
        self.is_maintaining_orbit = maintaining;
    }

    /// Add fuel to satellite
    pub fn add_fuel(&mut self, amount: f32) {
        self.current_fuel = (self.current_fuel + amount).min(self.max_fuel);
        self.mass = self.base_mass + self.current_fuel;
    }

    /// Consume fuel for operations
    pub fn consume_fuel(&mut self, amount: f32) -> bool {
        if self.current_fuel >= amount {
            self.current_fuel -= amount;
            self.mass = self.base_mass + self.current_fuel;
            true
        } else {
            false
        }
    }

    /// Get satellite status color based on fuel level
    pub fn status_color(&self) -> Color {
        let fuel_percent = self.fuel_percentage();
        if fuel_percent > GameConstants::SATELLITE_EMERGENCY_FUEL_THRESHOLD * 100.0 {
            crate::game_constants::colors::SATELLITE_STATUS_ACTIVE
        } else if fuel_percent > GameConstants::SATELLITE_CRITICAL_FUEL_THRESHOLD * 100.0 {
            crate::game_constants::colors::SATELLITE_STATUS_LOW_FUEL
        } else if fuel_percent > 0.0 {
            crate::game_constants::colors::SATELLITE_STATUS_CRITICAL
        } else {
            crate::game_constants::colors::SATELLITE_STATUS_DEPLETED
        }
    }

    // === Position and Velocity Accessors ===
    // These are also in GameObject trait, but provided here for direct access

    pub fn position(&self) -> Vec2 {
        self.data.position
    }

    pub fn velocity(&self) -> Vec2 {
        self.data.velocity
    }

    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.data.velocity = velocity;
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.data.position = position;
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    // === Maintenance ===

    pub fn set_maintenance_interval(&mut self, interval: f32) {
        self.maintenance_interval = interval;
    }

    pub fn maintenance_interval(&self) -> f32 {
        self.maintenance_interval
    }

    pub fn last_maintenance_time(&self) -> f32 {
        self.last_maintenance_time
    }

    pub fn set_last_maintenance_time(&mut self, time: f32) {
        self.last_maintenance_time = time;
    }

    pub fn maintenance_fuel_reserve(&self) -> f32 {
        self.maintenance_fuel_reserve
    }

    pub fn set_maintenance_fuel_reserve(&mut self, reserve: f32) {
        self.maintenance_fuel_reserve = reserve;
    }

    // === Fuel Collection ===

    pub fn is_collecting_fuel(&self) -> bool {
        self.is_collecting_fuel
    }

    pub fn set_is_collecting_fuel(&mut self, collecting: bool) {
        self.is_collecting_fuel = collecting;
    }

    pub fn start_fuel_collection(&mut self, planet_id: usize) {
        self.is_collecting_fuel = true;
        self.fuel_source_planet_id = Some(planet_id);
    }

    pub fn stop_fuel_collection(&mut self) {
        self.is_collecting_fuel = false;
        self.fuel_source_planet_id = None;
    }

    pub fn fuel_source_planet_id(&self) -> Option<usize> {
        self.fuel_source_planet_id
    }

    pub fn set_fuel_source_planet_id(&mut self, planet_id: Option<usize>) {
        self.fuel_source_planet_id = planet_id;
    }

    pub fn collection_rate(&self) -> f32 {
        self.collection_rate
    }

    pub fn set_collection_rate(&mut self, rate: f32) {
        self.collection_rate = rate;
    }

    // === Network & Communication ===

    pub fn add_nearby_satellite(&mut self, satellite_id: usize) {
        if !self.nearby_satellites.contains(&satellite_id) {
            self.nearby_satellites.push(satellite_id);
        }
    }

    pub fn remove_nearby_satellite(&mut self, satellite_id: usize) {
        self.nearby_satellites.retain(|&id| id != satellite_id);
    }

    pub fn nearby_satellites(&self) -> &[usize] {
        &self.nearby_satellites
    }

    pub fn clear_nearby_satellites(&mut self) {
        self.nearby_satellites.clear();
    }

    pub fn add_nearby_rocket(&mut self, rocket_id: usize) {
        if !self.nearby_rockets.contains(&rocket_id) {
            self.nearby_rockets.push(rocket_id);
        }
    }

    pub fn remove_nearby_rocket(&mut self, rocket_id: usize) {
        self.nearby_rockets.retain(|&id| id != rocket_id);
    }

    pub fn nearby_rockets(&self) -> &[usize] {
        &self.nearby_rockets
    }

    pub fn clear_nearby_rockets(&mut self) {
        self.nearby_rockets.clear();
    }

    pub fn transfer_range(&self) -> f32 {
        self.transfer_range
    }

    pub fn set_transfer_range(&mut self, range: f32) {
        self.transfer_range = range;
    }

    pub fn is_transferring_fuel(&self) -> bool {
        self.is_transferring_fuel
    }

    pub fn set_transferring_fuel(&mut self, transferring: bool) {
        self.is_transferring_fuel = transferring;
    }

    /// Transfer fuel to another satellite or rocket
    pub fn transfer_fuel(&mut self, amount: f32) -> f32 {
        // Reserve maintenance fuel
        let available_fuel = (self.current_fuel - self.maintenance_fuel_reserve).max(0.0);
        let transfer_amount = amount.min(available_fuel);

        if transfer_amount > 0.0 {
            self.consume_fuel(transfer_amount);
            transfer_amount
        } else {
            0.0
        }
    }
}

impl GameObject for Satellite {
    fn update(&mut self, delta_time: f32) {
        // Update position
        self.data.position += self.data.velocity * delta_time;

        // Update rotation for visual effect
        self.rotation += delta_time * 0.5; // Slow rotation

        // Orbital maintenance logic is handled by OrbitMaintenance system in SatelliteManager
        // Automatic fuel collection is handled by SatelliteManager

        // Update maintenance time
        self.last_maintenance_time += delta_time;
    }

    fn draw(&self) {
        // Draw body
        draw_circle(
            self.data.position.x,
            self.data.position.y,
            GameConstants::SATELLITE_SIZE,
            self.data.color,
        );

        // Draw solar panels (simple rectangles)
        let panel_offset = GameConstants::SATELLITE_SIZE + 2.0;
        let panel_width = GameConstants::SATELLITE_PANEL_SIZE;
        let panel_height = 2.0;

        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // Left panel - extends to the left from satellite body
        let left_panel_center = Vec2::new(-panel_offset - panel_width / 2.0, 0.0);
        let rotated_left = Vec2::new(
            left_panel_center.x * cos_r - left_panel_center.y * sin_r,
            left_panel_center.x * sin_r + left_panel_center.y * cos_r,
        );
        let left_panel_pos = self.data.position + rotated_left;

        draw_rectangle(
            left_panel_pos.x - panel_width / 2.0,
            left_panel_pos.y - panel_height / 2.0,
            panel_width,
            panel_height,
            crate::game_constants::colors::SATELLITE_PANEL_COLOR,
        );

        // Right panel - extends to the right from satellite body
        let right_panel_center = Vec2::new(panel_offset + panel_width / 2.0, 0.0);
        let rotated_right = Vec2::new(
            right_panel_center.x * cos_r - right_panel_center.y * sin_r,
            right_panel_center.x * sin_r + right_panel_center.y * cos_r,
        );
        let right_panel_pos = self.data.position + rotated_right;

        draw_rectangle(
            right_panel_pos.x - panel_width / 2.0,
            right_panel_pos.y - panel_height / 2.0,
            panel_width,
            panel_height,
            crate::game_constants::colors::SATELLITE_PANEL_COLOR,
        );

        // Draw status indicator (small circle at center)
        let status_radius = 3.0;
        draw_circle(
            self.data.position.x,
            self.data.position.y,
            status_radius,
            self.status_color(),
        );
    }

    fn position(&self) -> Vec2 {
        self.data.position
    }

    fn velocity(&self) -> Vec2 {
        self.data.velocity
    }

    fn set_velocity(&mut self, velocity: Vec2) {
        self.data.velocity = velocity;
    }

    fn color(&self) -> Color {
        self.data.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satellite_creation() {
        let satellite = Satellite::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Color::new(0.0, 1.0, 1.0, 1.0), // CYAN
        );
        assert_eq!(satellite.current_fuel(), GameConstants::SATELLITE_STARTING_FUEL);
    }

    #[test]
    fn test_satellite_from_rocket_conversion() {
        let rocket_fuel = 100.0;
        let satellite = Satellite::from_rocket(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            rocket_fuel,
        );

        let expected_fuel = rocket_fuel * GameConstants::SATELLITE_CONVERSION_FUEL_RETENTION;
        assert_eq!(satellite.current_fuel(), expected_fuel.min(GameConstants::SATELLITE_MAX_FUEL));
    }

    #[test]
    fn test_fuel_operations() {
        let mut satellite = Satellite::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.0),
            Color::new(0.0, 1.0, 1.0, 1.0), // CYAN
        );

        let initial_fuel = satellite.current_fuel();
        satellite.add_fuel(10.0);
        assert_eq!(satellite.current_fuel(), initial_fuel + 10.0);

        assert!(satellite.consume_fuel(5.0));
        assert_eq!(satellite.current_fuel(), initial_fuel + 5.0);

        // Try to consume more than available
        assert!(!satellite.consume_fuel(1000.0));
    }
}
