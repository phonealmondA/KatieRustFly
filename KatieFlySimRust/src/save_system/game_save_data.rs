// Game Save Data - Serializable game state snapshot
// Used for both save files and network packets (multiplayer)
// Uses bincode for compact binary serialization

use serde::{Deserialize, Serialize};
use macroquad::prelude::*;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

use crate::entities::{Planet, Rocket, Satellite, Bullet};
use crate::systems::EntityId;

/// Serializable Vec2 wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedVector2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for SavedVector2 {
    fn from(v: Vec2) -> Self {
        SavedVector2 { x: v.x, y: v.y }
    }
}

impl From<SavedVector2> for Vec2 {
    fn from(v: SavedVector2) -> Self {
        Vec2::new(v.x, v.y)
    }
}

/// Saved planet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlanet {
    pub id: EntityId,
    pub position: SavedVector2,
    pub velocity: SavedVector2,
    pub mass: f32,
    pub radius: f32,
    pub initial_mass: Option<f32>,   // Original mass (for proportional scaling after load)
    pub initial_radius: Option<f32>, // Original radius (for proportional scaling after load)
    pub color: (u8, u8, u8), // RGB
}

impl SavedPlanet {
    pub fn from_planet(id: EntityId, planet: &Planet) -> Self {
        use crate::entities::GameObject;

        SavedPlanet {
            id,
            position: planet.position().into(),
            velocity: planet.velocity().into(),
            mass: planet.mass(),
            radius: planet.radius(),
            initial_mass: Some(planet.initial_mass()),
            initial_radius: Some(planet.initial_radius()),
            color: (
                (planet.color().r * 255.0) as u8,
                (planet.color().g * 255.0) as u8,
                (planet.color().b * 255.0) as u8,
            ),
        }
    }

    pub fn to_planet(&self) -> (EntityId, Planet) {
        // If initial values are saved, use them; otherwise use current values as initial
        let initial_mass = self.initial_mass.unwrap_or(self.mass);
        let initial_radius = self.initial_radius.unwrap_or(self.radius);

        let mut planet = Planet::new_with_initials(
            self.position.clone().into(),
            self.radius,
            self.mass,
            initial_radius,
            initial_mass,
            Color::from_rgba(self.color.0, self.color.1, self.color.2, 255),
        );

        // Restore velocity (critical for orbital mechanics!)
        planet.set_velocity(self.velocity.clone().into());

        (self.id, planet)
    }
}

/// Saved rocket data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRocket {
    pub id: EntityId,
    pub position: SavedVector2,
    pub velocity: SavedVector2,
    pub rotation: f32,
    pub fuel: f32,
    pub color: (u8, u8, u8),
    pub player_id: Option<u32>, // Which player owns this rocket (for multiplayer)
}

impl SavedRocket {
    pub fn from_rocket(id: EntityId, rocket: &Rocket) -> Self {
        use crate::entities::GameObject;

        SavedRocket {
            id,
            position: rocket.position().into(),
            velocity: rocket.velocity().into(),
            rotation: rocket.rotation(),
            fuel: rocket.current_fuel(),
            color: (
                (rocket.color().r * 255.0) as u8,
                (rocket.color().g * 255.0) as u8,
                (rocket.color().b * 255.0) as u8,
            ),
            player_id: rocket.player_id(),
        }
    }

    pub fn to_rocket(&self) -> (EntityId, Rocket) {
        use crate::game_constants::GameConstants;

        let mut rocket = Rocket::new(
            self.position.clone().into(),
            self.velocity.clone().into(),
            Color::from_rgba(self.color.0, self.color.1, self.color.2, 255),
            GameConstants::ROCKET_BASE_MASS,
        );

        // Use set_fuel_direct to avoid momentum preservation during load
        // (velocity is already correct from the save, changing mass shouldn't affect it)
        rocket.set_fuel(self.fuel);
        rocket.set_rotation(self.rotation);
        rocket.set_player_id(self.player_id);

        (self.id, rocket)
    }
}

/// Saved satellite data (extended for comprehensive state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSatellite {
    pub id: EntityId,
    pub position: SavedVector2,
    pub velocity: SavedVector2,
    pub rotation: f32,
    pub fuel: f32,

    // Orbital maintenance state
    pub target_orbit_radius: f32,
    pub is_maintaining_orbit: bool,
    pub last_maintenance_time: f32,
    pub maintenance_interval: f32,
    pub maintenance_fuel_reserve: f32,

    // Fuel collection state
    pub is_collecting_fuel: bool,
    pub fuel_source_planet_id: Option<usize>,
    pub collection_rate: f32,

    // Network configuration
    pub transfer_range: f32,
}

impl SavedSatellite {
    pub fn from_satellite(id: EntityId, satellite: &Satellite) -> Self {
        use crate::entities::GameObject;

        SavedSatellite {
            id,
            position: satellite.position().into(),
            velocity: satellite.velocity().into(),
            rotation: satellite.rotation(),
            fuel: satellite.current_fuel(),
            target_orbit_radius: satellite.target_orbit_radius(),
            is_maintaining_orbit: satellite.is_maintaining_orbit(),
            last_maintenance_time: satellite.last_maintenance_time(),
            maintenance_interval: satellite.maintenance_interval(),
            maintenance_fuel_reserve: satellite.maintenance_fuel_reserve(),
            is_collecting_fuel: satellite.is_collecting_fuel(),
            fuel_source_planet_id: satellite.fuel_source_planet_id(),
            collection_rate: satellite.collection_rate(),
            transfer_range: satellite.transfer_range(),
        }
    }

    pub fn to_satellite(&self) -> (EntityId, Satellite) {
        use crate::game_constants::colors;

        let mut satellite = Satellite::new(
            self.position.clone().into(),
            self.velocity.clone().into(),
            colors::SATELLITE_BODY_COLOR,
        );

        // Restore fuel
        satellite.add_fuel(self.fuel - satellite.current_fuel());

        // Restore orbital maintenance state
        satellite.set_target_orbit_radius(self.target_orbit_radius);
        satellite.set_is_maintaining_orbit(self.is_maintaining_orbit);
        satellite.set_last_maintenance_time(self.last_maintenance_time);
        satellite.set_maintenance_interval(self.maintenance_interval);
        satellite.set_maintenance_fuel_reserve(self.maintenance_fuel_reserve);

        // Restore fuel collection state
        satellite.set_is_collecting_fuel(self.is_collecting_fuel);
        satellite.set_fuel_source_planet_id(self.fuel_source_planet_id);
        satellite.set_collection_rate(self.collection_rate);

        // Restore network configuration
        satellite.set_transfer_range(self.transfer_range);

        (self.id, satellite)
    }
}

/// Saved bullet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedBullet {
    pub id: EntityId,
    pub position: SavedVector2,
    pub velocity: SavedVector2,
    pub mass: f32,
    pub lifetime: f32,
    pub color: (u8, u8, u8),
}

impl SavedBullet {
    pub fn from_bullet(id: EntityId, bullet: &Bullet) -> Self {
        use crate::entities::GameObject;

        SavedBullet {
            id,
            position: bullet.position().into(),
            velocity: bullet.velocity().into(),
            mass: bullet.mass(),
            lifetime: bullet.lifetime(),
            color: (
                (bullet.color().r * 255.0) as u8,
                (bullet.color().g * 255.0) as u8,
                (bullet.color().b * 255.0) as u8,
            ),
        }
    }

    pub fn to_bullet(&self) -> (EntityId, Bullet) {
        let mut bullet = Bullet::new(
            self.position.clone().into(),
            self.velocity.clone().into(),
        );

        // Restore lifetime (critical for bullets to maintain their age across network)
        bullet.set_lifetime(self.lifetime);

        (self.id, bullet)
    }
}

/// Camera save data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCamera {
    pub center: SavedVector2,
    pub zoom: f32,
}

/// Complete game save data / state snapshot
/// Used for both save files (disk) and network packets (multiplayer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSaveData {
    pub version: u32,
    pub timestamp_secs: u64,        // Unix timestamp for ordering
    pub game_time: f32,              // Elapsed game time

    // World state
    pub planets: Vec<SavedPlanet>,
    pub rockets: Vec<SavedRocket>,
    pub satellites: Vec<SavedSatellite>,
    pub bullets: Vec<SavedBullet>,

    // Player state (multiplayer support)
    pub player_id: Option<u32>,      // None = single player, 0-19 = multiplayer
    pub active_rocket_id: Option<EntityId>,
    pub player_names: HashMap<u32, String>, // Map player IDs to player names (for multiplayer)

    // Camera (per-client, not synced in multiplayer)
    pub camera: SavedCamera,

    // Map configuration
    pub map_name: Option<String>,    // Which map is being played (e.g., "earth moon", "solar 1")
}

impl GameSaveData {
    pub fn new() -> Self {
        GameSaveData {
            version: 1,
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            game_time: 0.0,
            planets: Vec::new(),
            rockets: Vec::new(),
            satellites: Vec::new(),
            bullets: Vec::new(),
            player_id: None,  // Single player by default
            active_rocket_id: None,
            player_names: HashMap::new(), // Empty by default
            camera: SavedCamera {
                center: SavedVector2 { x: 0.0, y: 0.0 },
                zoom: 1.0,
            },
            map_name: None,   // No map specified by default
        }
    }

    /// Save to binary file using bincode
    /// Binary format is compact and fast - perfect for both saves and network packets
    pub fn save_to_file(&self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create saves directory if it doesn't exist
        fs::create_dir_all("saves")?;

        let file_path = format!("saves/{}.sav", save_name);
        let bytes = bincode::serialize(self)?;
        let byte_count = bytes.len();
        fs::write(&file_path, bytes)?;

        log::info!("Game saved to: {} ({} bytes)", file_path, byte_count);
        Ok(())
    }

    /// Save to multiplayer saves folder (saves/multi/)
    pub fn save_to_multi_file(&self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create saves/multi directory if it doesn't exist
        fs::create_dir_all("saves/multi")?;

        let file_path = format!("saves/multi/{}.sav", save_name);
        let bytes = bincode::serialize(self)?;
        let byte_count = bytes.len();
        fs::write(&file_path, bytes)?;

        log::info!("Multiplayer game saved to: {} ({} bytes)", file_path, byte_count);
        Ok(())
    }

    /// Load from binary file using bincode
    pub fn load_from_file(save_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = format!("saves/{}.sav", save_name);

        if !Path::new(&file_path).exists() {
            return Err(format!("Save file not found: {}", file_path).into());
        }

        let bytes = fs::read(&file_path)?;
        let save_data: GameSaveData = bincode::deserialize(&bytes)?;

        log::info!("Game loaded from: {} ({} bytes)", file_path, bytes.len());
        Ok(save_data)
    }

    /// Load from multiplayer saves folder (saves/multi/)
    pub fn load_from_multi_file(save_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = format!("saves/multi/{}.sav", save_name);

        if !Path::new(&file_path).exists() {
            return Err(format!("Multiplayer save file not found: {}", file_path).into());
        }

        let bytes = fs::read(&file_path)?;
        let save_data: GameSaveData = bincode::deserialize(&bytes)?;

        log::info!("Multiplayer game loaded from: {} ({} bytes)", file_path, bytes.len());
        Ok(save_data)
    }

    /// Delete a save file
    pub fn delete_save(save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = format!("saves/{}.sav", save_name);
        fs::remove_file(&file_path)?;
        log::info!("Deleted save: {}", file_path);
        Ok(())
    }

    /// Check if a save exists
    pub fn save_exists(save_name: &str) -> bool {
        let file_path = format!("saves/{}.sav", save_name);
        Path::new(&file_path).exists()
    }

    /// Serialize to bytes (for network packets)
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(bincode::serialize(self)?)
    }

    /// Deserialize from bytes (for network packets)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(bytes)?)
    }
}

impl Default for GameSaveData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector2_conversion() {
        let v = Vec2::new(100.0, 200.0);
        let saved: SavedVector2 = v.into();
        assert_eq!(saved.x, 100.0);
        assert_eq!(saved.y, 200.0);

        let restored: Vec2 = saved.into();
        assert_eq!(restored.x, 100.0);
        assert_eq!(restored.y, 200.0);
    }

    #[test]
    fn test_save_data_creation() {
        let save_data = GameSaveData::new();
        assert_eq!(save_data.version, 1);
        assert_eq!(save_data.game_time, 0.0);
        assert!(save_data.planets.is_empty());
    }

    #[test]
    fn test_serialization() {
        let save_data = GameSaveData::new();

        // Test bincode serialization
        let bytes = bincode::serialize(&save_data);
        assert!(bytes.is_ok());

        let deserialized: Result<GameSaveData, _> = bincode::deserialize(&bytes.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_to_from_bytes() {
        let save_data = GameSaveData::new();

        // Test to_bytes/from_bytes methods (for network packets)
        let bytes = save_data.to_bytes();
        assert!(bytes.is_ok());

        let restored = GameSaveData::from_bytes(&bytes.unwrap());
        assert!(restored.is_ok());

        let restored_data = restored.unwrap();
        assert_eq!(restored_data.version, save_data.version);
        assert_eq!(restored_data.player_id, None);
    }
}
