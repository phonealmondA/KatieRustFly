pub mod maps;
pub mod orbit_calculator;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

/// Serializable wrapper for macroquad Color
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<Color> for SerializableColor {
    fn from(color: Color) -> Self {
        SerializableColor {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

impl From<SerializableColor> for Color {
    fn from(sc: SerializableColor) -> Self {
        Color::new(sc.r, sc.g, sc.b, sc.a)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapConfiguration {
    pub name: String,
    pub description: String,
    pub celestial_bodies: Vec<CelestialBodyConfig>,
    pub player_spawn_body_index: usize, // Which body to spawn on
    pub central_body_index: Option<usize>, // Which body is the center (if any)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CelestialBodyConfig {
    pub name: String,
    pub mass: f32,
    pub radius: f32,
    #[serde(with = "color_serde")]
    pub color: Color,
    pub orbital_parent_index: Option<usize>, // None = stationary, Some(i) = orbits body i
    pub orbital_distance: Option<f32>, // Distance from parent
    pub orbital_period: Option<f32>, // Seconds to complete orbit
    pub initial_angle: f32, // Starting angle in radians (0 = right, π/2 = up)
    pub is_pinned: bool, // If true, doesn't move (for central bodies)
}

// Custom serde module for Color
mod color_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sc = SerializableColor::from(*color);
        sc.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sc = SerializableColor::deserialize(deserializer)?;
        Ok(Color::from(sc))
    }
}

impl MapConfiguration {
    pub fn get_spawn_body(&self) -> &CelestialBodyConfig {
        &self.celestial_bodies[self.player_spawn_body_index]
    }

    /// Load a map from a RON file
    pub fn load_from_file(filename: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(filename)
            .map_err(|e| format!("Failed to read file {}: {}", filename, e))?;

        let map: MapConfiguration = ron::from_str(&contents)
            .map_err(|e| format!("Failed to parse RON from {}: {}", filename, e))?;

        Ok(map)
    }

    /// Save a map to a RON file
    pub fn save_to_file(&self, filename: &str) -> Result<(), String> {
        let ron_string = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize map: {}", e))?;

        std::fs::write(filename, ron_string)
            .map_err(|e| format!("Failed to write file {}: {}", filename, e))?;

        Ok(())
    }
}
