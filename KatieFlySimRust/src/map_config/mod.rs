pub mod maps;
pub mod orbit_calculator;

use macroquad::prelude::*;

#[derive(Clone, Debug)]
pub struct MapConfiguration {
    pub name: String,
    pub description: String,
    pub celestial_bodies: Vec<CelestialBodyConfig>,
    pub player_spawn_body_index: usize, // Which body to spawn on
    pub central_body_index: Option<usize>, // Which body is the center (if any)
}

#[derive(Clone, Debug)]
pub struct CelestialBodyConfig {
    pub name: String,
    pub mass: f32,
    pub radius: f32,
    pub color: Color,
    pub orbital_parent_index: Option<usize>, // None = stationary, Some(i) = orbits body i
    pub orbital_distance: Option<f32>, // Distance from parent
    pub orbital_period: Option<f32>, // Seconds to complete orbit
    pub initial_angle: f32, // Starting angle in radians (0 = right, π/2 = up)
    pub is_pinned: bool, // If true, doesn't move (for central bodies)
}

impl MapConfiguration {
    pub fn get_spawn_body(&self) -> &CelestialBodyConfig {
        &self.celestial_bodies[self.player_spawn_body_index]
    }
}
