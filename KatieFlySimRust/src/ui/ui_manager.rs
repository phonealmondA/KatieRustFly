// UI Manager - Centralized UI rendering and coordination
// Manages information panels, visualizations, and UI layout

use macroquad::prelude::*;
use std::collections::HashMap;

use crate::entities::{Rocket, Planet, Satellite};
use crate::systems::{EntityId, SatelliteManager};
use crate::ui::TextPanel;

/// UI Manager - Coordinates all UI rendering
pub struct UIManager {
    // Font configuration
    font_size: f32,

    // Visualization flags
    pub show_fuel_collection_lines: bool,
    pub show_satellite_network_lines: bool,
    pub show_satellite_fuel_transfers: bool,
    pub show_satellite_to_rocket_lines: bool,

    // Color configuration
    fuel_collection_color: Color,
    network_line_color: Color,
    transfer_line_color: Color,

    // Line thickness
    line_thickness: f32,
}

impl UIManager {
    pub fn new() -> Self {
        UIManager {
            font_size: 16.0,
            show_fuel_collection_lines: true,
            show_satellite_network_lines: true,
            show_satellite_fuel_transfers: true,
            show_satellite_to_rocket_lines: true,
            fuel_collection_color: Color::new(0.0, 1.0, 0.5, 0.4),
            network_line_color: Color::new(0.0, 0.8, 1.0, 0.3),
            transfer_line_color: Color::new(1.0, 0.8, 0.0, 0.5),
            line_thickness: 2.0,
        }
    }

    // === Configuration ===

    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    // === Window Management ===

    pub fn handle_window_resize(&mut self, _width: f32, _height: f32) {
        // Handle UI adjustments on window resize
        // Panels will automatically adjust based on screen dimensions
    }

    // === Visualization Drawing ===

    /// Draw fuel collection lines from rockets to planets
    pub fn draw_fuel_collection_lines(
        &self,
        rockets: &HashMap<EntityId, &Rocket>,
        planets: &HashMap<EntityId, &Planet>,
    ) {
        if !self.show_fuel_collection_lines {
            return;
        }

        for rocket in rockets.values() {
            // Find nearest planet within collection range
            let mut nearest_planet: Option<&Planet> = None;
            let mut min_distance = f32::MAX;

            for planet in planets.values() {
                let distance = (rocket.position() - planet.position()).length();
                if distance < min_distance && distance <= planet.fuel_collection_range() {
                    min_distance = distance;
                    nearest_planet = Some(planet);
                }
            }

            if let Some(planet) = nearest_planet {
                // Draw line from rocket to planet
                draw_line(
                    rocket.position().x,
                    rocket.position().y,
                    planet.position().x,
                    planet.position().y,
                    self.line_thickness,
                    self.fuel_collection_color,
                );

                // Draw fuel icon/indicator
                let mid_point = (rocket.position() + planet.position()) * 0.5;
                draw_circle(mid_point.x, mid_point.y, 4.0, self.fuel_collection_color);
            }
        }
    }

    /// Draw multiple fuel collection lines with efficiency indicators
    pub fn draw_multiple_fuel_lines(
        &self,
        rocket_position: Vec2,
        fuel_sources: &[(Vec2, f32)], // (position, efficiency)
    ) {
        if !self.show_fuel_collection_lines {
            return;
        }

        for (source_pos, efficiency) in fuel_sources {
            let color = Color::new(
                1.0 - efficiency,
                *efficiency,
                0.0,
                0.4,
            );

            draw_line(
                rocket_position.x,
                rocket_position.y,
                source_pos.x,
                source_pos.y,
                self.line_thickness,
                color,
            );
        }
    }

    /// Draw satellite network connection lines
    pub fn draw_satellite_network_lines(
        &self,
        satellites: &HashMap<EntityId, &Satellite>,
        max_range: f32,
    ) {
        if !self.show_satellite_network_lines {
            return;
        }

        let satellite_list: Vec<(EntityId, &Satellite)> = satellites
            .iter()
            .map(|(id, sat)| (*id, *sat))
            .collect();

        for i in 0..satellite_list.len() {
            for j in (i + 1)..satellite_list.len() {
                let (_, sat1) = satellite_list[i];
                let (_, sat2) = satellite_list[j];

                let distance = (sat1.position() - sat2.position()).length();

                if distance <= max_range {
                    // Color based on distance (closer = stronger connection)
                    let strength = 1.0 - (distance / max_range);
                    let color = Color::new(
                        0.0,
                        0.8 * strength,
                        1.0,
                        0.3 * strength,
                    );

                    draw_line(
                        sat1.position().x,
                        sat1.position().y,
                        sat2.position().x,
                        sat2.position().y,
                        self.line_thickness,
                        color,
                    );
                }
            }
        }
    }

    /// Draw active fuel transfers from satellites to planets
    pub fn draw_satellite_fuel_transfers(
        &self,
        satellites: &HashMap<EntityId, &Satellite>,
        planets: &HashMap<EntityId, &Planet>,
    ) {
        if !self.show_satellite_fuel_transfers {
            return;
        }

        for satellite in satellites.values() {
            if let Some(planet_id) = satellite.fuel_source_planet_id() {
                // Find the planet
                if let Some(planet) = planets.get(&planet_id) {
                    // Draw animated transfer line
                    self.draw_animated_transfer_line(
                        satellite.position(),
                        planet.position(),
                        self.transfer_line_color,
                    );
                }
            }
        }
    }

    /// Draw fuel transfer lines from satellites to rockets
    pub fn draw_satellite_to_rocket_lines(
        &self,
        satellites: &HashMap<EntityId, &Satellite>,
        rockets: &HashMap<EntityId, &Rocket>,
    ) {
        if !self.show_satellite_to_rocket_lines {
            return;
        }

        for satellite in satellites.values() {
            if satellite.is_transferring_fuel() {
                // Find nearby rockets
                for rocket in rockets.values() {
                    let distance = (satellite.position() - rocket.position()).length();

                    if distance <= satellite.transfer_range() {
                        // Draw transfer line
                        self.draw_animated_transfer_line(
                            satellite.position(),
                            rocket.position(),
                            Color::new(0.0, 1.0, 1.0, 0.6),
                        );
                    }
                }
            }
        }
    }

    /// Draw animated transfer line with particle effect
    fn draw_animated_transfer_line(
        &self,
        from: Vec2,
        to: Vec2,
        color: Color,
    ) {
        // Draw base line
        draw_line(
            from.x,
            from.y,
            to.x,
            to.y,
            self.line_thickness * 1.5,
            color,
        );

        // Draw animated particles along the line
        let time = get_time() as f32;
        let num_particles = 5;

        for i in 0..num_particles {
            let offset = (time * 2.0 + i as f32 * 0.2) % 1.0;
            let particle_pos = from + (to - from) * offset;

            draw_circle(
                particle_pos.x,
                particle_pos.y,
                3.0,
                Color::new(color.r, color.g, color.b, 0.8),
            );
        }
    }

    /// Draw all visualizations
    pub fn draw_all_visualizations(
        &self,
        rockets: &HashMap<EntityId, &Rocket>,
        satellites: &HashMap<EntityId, &Satellite>,
        planets: &HashMap<EntityId, &Planet>,
        satellite_manager: &SatelliteManager,
    ) {
        // Draw in order of layering (background to foreground)
        self.draw_satellite_network_lines(
            satellites,
            satellite_manager.config().max_transfer_range,
        );

        self.draw_fuel_collection_lines(rockets, planets);
        self.draw_satellite_fuel_transfers(satellites, planets);
        self.draw_satellite_to_rocket_lines(satellites, rockets);
    }

    // === Toggle Methods ===

    pub fn toggle_fuel_collection_lines(&mut self) {
        self.show_fuel_collection_lines = !self.show_fuel_collection_lines;
    }

    pub fn toggle_satellite_network_lines(&mut self) {
        self.show_satellite_network_lines = !self.show_satellite_network_lines;
    }

    pub fn toggle_satellite_fuel_transfers(&mut self) {
        self.show_satellite_fuel_transfers = !self.show_satellite_fuel_transfers;
    }

    pub fn toggle_satellite_to_rocket_lines(&mut self) {
        self.show_satellite_to_rocket_lines = !self.show_satellite_to_rocket_lines;
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_manager_creation() {
        let ui_manager = UIManager::new();
        assert_eq!(ui_manager.font_size(), 16.0);
        assert!(ui_manager.show_fuel_collection_lines);
        assert!(ui_manager.show_satellite_network_lines);
    }

    #[test]
    fn test_toggle_visualizations() {
        let mut ui_manager = UIManager::new();

        ui_manager.toggle_fuel_collection_lines();
        assert!(!ui_manager.show_fuel_collection_lines);

        ui_manager.toggle_fuel_collection_lines();
        assert!(ui_manager.show_fuel_collection_lines);
    }

    #[test]
    fn test_font_size_configuration() {
        let mut ui_manager = UIManager::new();

        ui_manager.set_font_size(20.0);
        assert_eq!(ui_manager.font_size(), 20.0);
    }
}
