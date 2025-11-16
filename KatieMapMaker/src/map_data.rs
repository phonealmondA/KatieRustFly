use serde::{Deserialize, Serialize};
use macroquad::prelude::*;

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
    pub player_spawn_body_index: usize,
    pub central_body_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CelestialBodyConfig {
    pub name: String,
    pub mass: f32,
    pub radius: f32,
    pub color: SerializableColor,
    pub orbital_parent_index: Option<usize>,
    pub orbital_distance: Option<f32>,
    pub orbital_period: Option<f32>,
    pub initial_angle: f32,
    pub is_pinned: bool,
}

impl MapConfiguration {
    pub fn new_empty() -> Self {
        MapConfiguration {
            name: String::from("New Map"),
            description: String::from("A custom map"),
            celestial_bodies: Vec::new(),
            player_spawn_body_index: 0,
            central_body_index: None,
        }
    }

    pub fn save_to_file(&self, filename: &str) -> Result<(), String> {
        let ron_string = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to serialize map: {}", e))?;

        std::fs::write(filename, ron_string)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    pub fn load_from_file(filename: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(filename)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let map: MapConfiguration = ron::from_str(&contents)
            .map_err(|e| format!("Failed to parse RON: {}", e))?;

        Ok(map)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Map name cannot be empty".to_string());
        }

        if self.celestial_bodies.is_empty() {
            return Err("Map must have at least one celestial body".to_string());
        }

        if self.player_spawn_body_index >= self.celestial_bodies.len() {
            return Err("Invalid player spawn body index".to_string());
        }

        if let Some(idx) = self.central_body_index {
            if idx >= self.celestial_bodies.len() {
                return Err("Invalid central body index".to_string());
            }
        }

        for (i, body) in self.celestial_bodies.iter().enumerate() {
            if body.mass <= 0.0 {
                return Err(format!("Body '{}' has invalid mass", body.name));
            }
            if body.radius <= 0.0 {
                return Err(format!("Body '{}' has invalid radius", body.name));
            }
            if let Some(parent_idx) = body.orbital_parent_index {
                if parent_idx >= self.celestial_bodies.len() {
                    return Err(format!("Body '{}' has invalid parent index", body.name));
                }
                if parent_idx == i {
                    return Err(format!("Body '{}' cannot orbit itself", body.name));
                }
            }
        }

        Ok(())
    }
}

impl CelestialBodyConfig {
    pub fn new_planet(name: &str) -> Self {
        CelestialBodyConfig {
            name: name.to_string(),
            mass: 100_000_000.0,
            radius: 5_000.0,
            color: SerializableColor {
                r: 0.5,
                g: 0.5,
                b: 0.8,
                a: 1.0,
            },
            orbital_parent_index: None,
            orbital_distance: None,
            orbital_period: None,
            initial_angle: 0.0,
            is_pinned: true,
        }
    }
}
