// Game Info Display - Comprehensive information panels for all game modes
// Displays 5 information panels: Rocket, Planet, Orbit, Controls, Network

use macroquad::prelude::*;

use crate::entities::{Rocket, Planet};
use crate::systems::{SatelliteNetworkStats, ReferenceBody};
use crate::ui::TextPanel;
use crate::utils::vector_helper;

/// Game mode for context-specific information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    SinglePlayer,
    SplitScreen,
    OnlineMultiplayer,
}

/// Network role for multiplayer games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkRole {
    None,
    Host,
    Client,
}

/// Game Info Display - Manages all information panels
pub struct GameInfoDisplay {
    // Panels
    rocket_panel: TextPanel,
    planet_panel: TextPanel,
    orbit_panel: TextPanel,
    network_panel: TextPanel,

    // Panel visibility
    show_rocket_panel: bool,
    show_planet_panel: bool,
    show_orbit_panel: bool,
    show_network_panel: bool,

    // Configuration
    panel_spacing: f32,
    panel_width: f32,
    panel_margin: f32,

    // Game context
    game_mode: GameMode,
    network_role: NetworkRole,

    // Rocket heading for visual indicator
    current_rocket_rotation: f32,

    // Player theme color (for split-screen)
    theme_color: Color,
}

impl GameInfoDisplay {
    pub fn new() -> Self {
        let panel_width = 280.0;
        let panel_margin = 10.0;
        let screen_height = screen_height();

        // Create panels with initial positions
        let rocket_panel = TextPanel::new(
            Vec2::new(panel_margin, panel_margin),
            Vec2::new(panel_width, 200.0),
        )
        .with_title("Rocket Info")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(0.0, 1.0, 0.5, 0.6));

        let planet_panel = TextPanel::new(
            Vec2::new(panel_margin, 220.0),
            Vec2::new(panel_width, 180.0),
        )
        .with_title("Nearest Planet")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(0.5, 0.5, 1.0, 0.6));

        let orbit_panel = TextPanel::new(
            Vec2::new(panel_margin, 410.0),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Orbital Info")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(1.0, 1.0, 0.0, 0.6));

        // Network panel on the right side
        let screen_width = screen_width();
        let network_panel = TextPanel::new(
            Vec2::new(screen_width - panel_width - panel_margin, panel_margin),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Network")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(1.0, 0.5, 0.0, 0.6));

        GameInfoDisplay {
            rocket_panel,
            planet_panel,
            orbit_panel,
            network_panel,
            show_rocket_panel: true,
            show_planet_panel: true,
            show_orbit_panel: true,
            show_network_panel: false,
            panel_spacing: 10.0,
            panel_width,
            panel_margin,
            game_mode: GameMode::SinglePlayer,
            network_role: NetworkRole::None,
            current_rocket_rotation: 0.0,
            theme_color: Color::new(0.3, 0.7, 1.0, 1.0),  // Default light blue
        }
    }

    /// Create a new GameInfoDisplay for a specific player in split-screen mode
    /// player_num: 0 for Player 1 (left, red), 1 for Player 2 (right, blue)
    pub fn new_for_player(player_num: usize) -> Self {
        let panel_width = 280.0;
        let panel_margin = 10.0;
        let screen_width = screen_width();
        let screen_height = screen_height();

        // Determine panel positions based on player
        let (x_pos, theme_color, player_name) = if player_num == 0 {
            // Player 1: left side, red theme
            (panel_margin, Color::new(1.0, 0.0, 0.0, 0.6), "Player 1")
        } else {
            // Player 2: right side, blue theme
            (screen_width - panel_width - panel_margin, Color::new(0.0, 0.5, 1.0, 0.6), "Player 2")
        };

        // Create rocket panel with player-specific position and color
        let rocket_panel = TextPanel::new(
            Vec2::new(x_pos, panel_margin),
            Vec2::new(panel_width, 200.0),
        )
        .with_title(&format!("{} Rocket", player_name))
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(theme_color);

        // Planet panel
        let planet_panel = TextPanel::new(
            Vec2::new(x_pos, 220.0),
            Vec2::new(panel_width, 180.0),
        )
        .with_title("Nearest Planet")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(theme_color);

        // Orbit panel
        let orbit_panel = TextPanel::new(
            Vec2::new(x_pos, 410.0),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Orbital Info")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(theme_color);

        // Network panel at bottom middle (same for both players)
        let network_x = screen_width / 2.0 - panel_width / 2.0;
        let network_panel = TextPanel::new(
            Vec2::new(network_x, screen_height - 180.0),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Satellite Network")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(1.0, 0.5, 0.0, 0.6));

        GameInfoDisplay {
            rocket_panel,
            planet_panel,
            orbit_panel,
            network_panel,
            show_rocket_panel: true,
            show_planet_panel: true,
            show_orbit_panel: true,
            show_network_panel: true,    // Show network panel in split-screen
            panel_spacing: 10.0,
            panel_width,
            panel_margin,
            game_mode: GameMode::SplitScreen,
            network_role: NetworkRole::None,
            current_rocket_rotation: 0.0,
            theme_color,  // Use player-specific theme color
        }
    }

    // === Configuration ===

    pub fn set_game_mode(&mut self, mode: GameMode) {
        self.game_mode = mode;
        // Update network panel visibility based on mode
        self.show_network_panel = matches!(
            mode,
            GameMode::SplitScreen | GameMode::OnlineMultiplayer
        );
    }

    pub fn set_network_role(&mut self, role: NetworkRole) {
        self.network_role = role;
    }

    // === Panel Visibility ===

    pub fn toggle_rocket_panel(&mut self) {
        self.show_rocket_panel = !self.show_rocket_panel;
    }

    pub fn toggle_planet_panel(&mut self) {
        self.show_planet_panel = !self.show_planet_panel;
    }

    pub fn toggle_orbit_panel(&mut self) {
        self.show_orbit_panel = !self.show_orbit_panel;
    }

    pub fn toggle_network_panel(&mut self) {
        self.show_network_panel = !self.show_network_panel;
    }

    pub fn hide_all_panels(&mut self) {
        self.show_rocket_panel = false;
        self.show_planet_panel = false;
        self.show_orbit_panel = false;
        self.show_network_panel = false;
    }

    pub fn show_all_panels(&mut self) {
        self.show_rocket_panel = true;
        self.show_planet_panel = true;
        self.show_orbit_panel = true;
        self.show_network_panel = matches!(
            self.game_mode,
            GameMode::SplitScreen | GameMode::OnlineMultiplayer
        );
    }

    // === Update Methods ===

    /// Update rocket information panel
    pub fn update_rocket_panel(&mut self, rocket: &Rocket, selected_thrust: f32) {
        let info = self.generate_vehicle_info(rocket, selected_thrust);
        self.rocket_panel.set_text(&info);
        self.current_rocket_rotation = rocket.rotation();
    }

    /// Update planet information panel
    pub fn update_planet_panel(&mut self, rocket_position: Vec2, selected_planet: Option<&Planet>, reference_body: ReferenceBody) {
        let info = self.generate_planet_info(rocket_position, selected_planet, reference_body);
        self.planet_panel.set_text(&info);
    }

    /// Update orbital information panel
    pub fn update_orbit_panel(&mut self, rocket: &Rocket, selected_planet: Option<&Planet>, all_planets: &[&Planet], reference_body: ReferenceBody) {
        let info = self.generate_orbit_info(rocket, selected_planet, all_planets, reference_body);
        self.orbit_panel.set_text(&info);
    }

    /// Update network information panel
    pub fn update_network_panel(
        &mut self,
        connected: bool,
        player_id: Option<usize>,
        player_count: usize,
        satellite_stats: Option<&SatelliteNetworkStats>,
    ) {
        let info = self.generate_network_info(connected, player_id, player_count, satellite_stats);
        self.network_panel.set_text(&info);
    }

    /// Update all panels at once
    pub fn update_all_panels(
        &mut self,
        rocket: Option<&Rocket>,
        planets: &[&Planet],
        selected_planet: Option<&Planet>,
        reference_body: ReferenceBody,
        selected_thrust: f32,
        network_connected: bool,
        player_id: Option<usize>,
        player_count: usize,
        satellite_stats: Option<&SatelliteNetworkStats>,
    ) {
        if let Some(rocket) = rocket {
            let rocket_pos = rocket.position();
            self.update_rocket_panel(rocket, selected_thrust);

            // Use selected planet for panels 2 and 3
            self.update_planet_panel(rocket_pos, selected_planet, reference_body);
            self.update_orbit_panel(rocket, selected_planet, planets, reference_body);
        }

        if self.game_mode != GameMode::SinglePlayer {
            self.update_network_panel(
                network_connected,
                player_id,
                player_count,
                satellite_stats,
            );
        }
    }

    // === Information Generation ===

    /// Generate vehicle information text
    fn generate_vehicle_info(&self, rocket: &Rocket, selected_thrust: f32) -> String {
        let velocity = rocket.velocity();
        let speed = velocity.length();
        let fuel_percent = rocket.fuel_percentage();
        let mass = rocket.mass();
        let thrust_percent = rocket.thrust_level() * 100.0;
        let selected_percent = selected_thrust * 100.0;
        let rotation_deg = rocket.rotation() * 180.0 / std::f32::consts::PI;

        format!(
            "Speed: {:.1} m/s\n\
             Fuel: {:.1}%\n\
             Mass: {:.1} kg\n\
             Thrust Set: {:.0}%\n\
             Thrust Now: {:.0}%\n\
             Heading: {:.0}°",
            speed,
            fuel_percent,
            mass,
            selected_percent,
            thrust_percent,
            rotation_deg
        )
    }

    /// Generate planet information text
    fn generate_planet_info(&mut self, rocket_position: Vec2, selected_planet: Option<&Planet>, reference_body: ReferenceBody) -> String {
        if let Some(planet) = selected_planet {
            let distance = vector_helper::distance(rocket_position, planet.position());
            let mass = planet.mass();
            let radius = planet.radius();
            let fuel_range = planet.fuel_collection_range();

            // Determine planet name from reference body
            let planet_name = match reference_body {
                ReferenceBody::Earth => "Earth",
                ReferenceBody::Moon => "Moon",
            };

            // Update panel title dynamically
            let title = format!("Selected Planet: {}", planet_name);
            self.planet_panel.set_title(Some(title));

            format!(
                "Mass: {:.2e} kg\n\
                 Radius: {:.0} m\n\
                 Fuel Range: {:.0} m",
                mass, radius, fuel_range
            )
        } else {
            self.planet_panel.set_title(Some("Selected Planet".to_string()));
            "No planet selected".to_string()
        }
    }

    /// Generate orbital information text
    fn generate_orbit_info(&mut self, rocket: &Rocket, selected_planet: Option<&Planet>, all_planets: &[&Planet], reference_body: ReferenceBody) -> String {
        if let Some(planet) = selected_planet {
            let rocket_pos = rocket.position();
            let rocket_vel = rocket.velocity();
            let planet_pos = planet.position();
            let planet_mass = planet.mass();
            let planet_radius = planet.radius();

            // Determine planet name from reference body
            let planet_name = match reference_body {
                ReferenceBody::Earth => "Earth",
                ReferenceBody::Moon => "Moon",
            };

            // Update panel title dynamically
            let title = format!("Orbital Info of {}", planet_name);
            self.orbit_panel.set_title(Some(title));

            // Calculate periapsis and apoapsis
            let (periapsis, apoapsis) = self.calculate_orbital_apsides(
                rocket_pos,
                rocket_vel,
                planet_pos,
                planet_mass,
            );

            // Calculate drift from other bodies
            let drift = self.calculate_drift_from_other_bodies(
                rocket_pos,
                planet,
                all_planets,
            );

            format!(
                "Periapsis: {:.0} m\n\
                 Apoapsis: {:.0} m\n\
                 Drift Approx.: {:.2} m/s²",
                periapsis.max(0.0) - planet_radius,  // Altitude above surface
                apoapsis - planet_radius,            // Altitude above surface
                drift
            )
        } else {
            self.orbit_panel.set_title(Some("Orbital Info".to_string()));
            match self.game_mode {
                GameMode::SinglePlayer => {
                    "Press C to convert\nto satellite when\nin stable orbit".to_string()
                }
                GameMode::SplitScreen => {
                    "P1: T to convert\nP2: Y to convert".to_string()
                }
                GameMode::OnlineMultiplayer => {
                    "Satellites sync\nacross network".to_string()
                }
            }
        }
    }

    /// Generate network information text
    fn generate_network_info(
        &self,
        connected: bool,
        player_id: Option<usize>,
        player_count: usize,
        satellite_stats: Option<&SatelliteNetworkStats>,
    ) -> String {
        let status = if connected { "CONNECTED" } else { "DISCONNECTED" };

        let role_text = match self.network_role {
            NetworkRole::None => "None",
            NetworkRole::Host => "Host",
            NetworkRole::Client => "Client",
        };

        let player_text = if let Some(id) = player_id {
            format!("Player {}", id)
        } else {
            "Waiting...".to_string()
        };

        let mut info = format!(
            "Status: {}\n\
             Role: {}\n\
             ID: {}\n\
             Players: {}",
            status, role_text, player_text, player_count
        );

        if let Some(stats) = satellite_stats {
            info.push_str(&format!(
                "\n\nSatellites: {}\n\
                 Network Fuel: {:.0}",
                stats.total_satellites, stats.total_network_fuel
            ));
        }

        info
    }

    // === Helper Methods ===

    fn find_nearest_planet<'a>(position: Vec2, planets: &'a [&'a Planet]) -> Option<&'a Planet> {
        planets
            .iter()
            .min_by(|a, b| {
                let dist_a = vector_helper::distance(position, a.position());
                let dist_b = vector_helper::distance(position, b.position());
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .copied()
    }

    fn find_primary_planet<'a>(position: Vec2, planets: &'a [&'a Planet]) -> Option<&'a Planet> {
        // Primary planet is the nearest massive body
        Self::find_nearest_planet(position, planets)
    }

    fn calculate_orbital_velocity(&self, planet_mass: f32, orbital_radius: f32) -> f32 {
        let g = 6.674e-11 * 1e9; // Scaled gravitational constant
        (g * planet_mass / orbital_radius).sqrt()
    }

    fn calculate_escape_velocity(&self, planet_mass: f32, distance: f32) -> f32 {
        let g = 6.674e-11 * 1e9; // Scaled gravitational constant
        (2.0 * g * planet_mass / distance).sqrt()
    }

    /// Calculate periapsis and apoapsis of orbit relative to a planet
    /// Returns (periapsis_distance, apoapsis_distance) from planet center
    fn calculate_orbital_apsides(
        &self,
        rocket_pos: Vec2,
        rocket_vel: Vec2,
        planet_pos: Vec2,
        planet_mass: f32,
    ) -> (f32, f32) {
        // Use game's gravity constant (from GameConstants::G = 100.0)
        let g = 100.0;

        // Position and velocity relative to planet
        let r_vec = rocket_pos - planet_pos;
        let r = r_vec.length();
        let v = rocket_vel.length();

        // Specific orbital energy: E = v²/2 - μ/r where μ = G*M
        let mu = g * planet_mass;
        let specific_energy = (v * v) / 2.0 - mu / r;

        // Specific angular momentum: h = |r × v|
        // In 2D: h = r.x * v.y - r.y * v.x
        let h = r_vec.x * rocket_vel.y - r_vec.y * rocket_vel.x;
        let h_squared = h * h;

        // Semi-major axis: a = -μ / (2*E)
        let a = -mu / (2.0 * specific_energy);

        // Eccentricity: e = sqrt(1 + (2*E*h²)/μ²)
        let e_squared = 1.0 + (2.0 * specific_energy * h_squared) / (mu * mu);
        let e = if e_squared > 0.0 {
            e_squared.sqrt()
        } else {
            0.0 // Circular orbit
        };

        // Periapsis and apoapsis distances
        let periapsis = a * (1.0 - e);
        let apoapsis = a * (1.0 + e);

        // Clamp to reasonable values
        let periapsis = periapsis.max(0.0);
        let apoapsis = if apoapsis < 0.0 || e >= 1.0 {
            // Hyperbolic or parabolic trajectory (escaping)
            999999.0
        } else {
            apoapsis
        };

        (periapsis, apoapsis)
    }

    /// Calculate gravitational drift acceleration from other bodies (not the selected planet)
    fn calculate_drift_from_other_bodies(
        &self,
        rocket_pos: Vec2,
        selected_planet: &Planet,
        all_planets: &[&Planet],
    ) -> f32 {
        // Use game's gravity constant
        let g = 100.0;
        let rocket_mass = 1.0; // Doesn't matter for acceleration calculation

        let mut total_drift_accel = Vec2::ZERO;

        for planet in all_planets {
            // Skip the selected planet
            if planet.position() == selected_planet.position() {
                continue;
            }

            // Calculate gravitational acceleration from this other planet
            let diff = planet.position() - rocket_pos;
            let distance = diff.length();

            if distance > 0.0 {
                let force_magnitude = (g * planet.mass() * rocket_mass) / (distance * distance);
                let acceleration = force_magnitude / rocket_mass;
                let direction = diff / distance;
                total_drift_accel += direction * acceleration;
            }
        }

        // Return magnitude of total drift acceleration
        total_drift_accel.length()
    }

    // === Layout Management ===

    pub fn reposition_panels(&mut self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        // Left side panels
        self.rocket_panel.set_position(Vec2::new(self.panel_margin, self.panel_margin));
        self.planet_panel.set_position(Vec2::new(
            self.panel_margin,
            self.panel_margin + 210.0,
        ));
        self.orbit_panel.set_position(Vec2::new(
            self.panel_margin,
            self.panel_margin + 400.0,
        ));

        // Right side panel (network)
        self.network_panel.set_position(Vec2::new(
            screen_width - self.panel_width - self.panel_margin,
            self.panel_margin,
        ));
    }

    // === Drawing ===

    /// Draw a visual rocket triangle showing the current heading (from old Hud)
    fn draw_heading_indicator(&self, rotation: f32) {
        // Position in the rocket panel, to the right of the "Heading" text
        let panel_pos = self.rocket_panel.position();
        let center_x = panel_pos.x + 240.0;  // Right side of the panel
        let center_y = panel_pos.y + 175.0;   // Near bottom, aligned with "Heading:" text

        // Fixed size rocket triangle (independent of game zoom)
        let size = 20.0;

        // Define triangle points (pointing up in local space)
        // Tip of rocket
        let tip = Vec2::new(0.0, -size);
        // Left base
        let left = Vec2::new(-size * 0.4, size * 0.5);
        // Right base
        let right = Vec2::new(size * 0.4, size * 0.5);

        // Rotate points by rocket rotation (negate for correct rotation direction, add PI for correct base orientation)
        let adjusted_rotation = -rotation + std::f32::consts::PI;
        let cos_r = adjusted_rotation.cos();
        let sin_r = adjusted_rotation.sin();

        let rotate = |p: Vec2| -> Vec2 {
            Vec2::new(
                p.x * cos_r - p.y * sin_r,
                p.x * sin_r + p.y * cos_r,
            )
        };

        let tip_rotated = rotate(tip);
        let left_rotated = rotate(left);
        let right_rotated = rotate(right);

        // Translate to screen position
        let tip_screen = Vec2::new(center_x + tip_rotated.x, center_y + tip_rotated.y);
        let left_screen = Vec2::new(center_x + left_rotated.x, center_y + left_rotated.y);
        let right_screen = Vec2::new(center_x + right_rotated.x, center_y + right_rotated.y);

        // Draw filled triangle with player theme color
        draw_triangle(
            tip_screen,
            left_screen,
            right_screen,
            self.theme_color,
        );

        // Draw outline for better visibility
        draw_line(tip_screen.x, tip_screen.y, left_screen.x, left_screen.y, 2.0, WHITE);
        draw_line(left_screen.x, left_screen.y, right_screen.x, right_screen.y, 2.0, WHITE);
        draw_line(right_screen.x, right_screen.y, tip_screen.x, tip_screen.y, 2.0, WHITE);
    }

    pub fn draw_all_panels(&self) {
        if self.show_rocket_panel {
            self.rocket_panel.draw();
            // Draw heading indicator after the panel
            self.draw_heading_indicator(self.current_rocket_rotation);
        }

        if self.show_planet_panel {
            self.planet_panel.draw();
        }

        if self.show_orbit_panel {
            self.orbit_panel.draw();
        }

        if self.show_network_panel {
            self.network_panel.draw();
        }
    }
}

impl Default for GameInfoDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_info_display_creation() {
        let display = GameInfoDisplay::new();
        assert!(display.show_rocket_panel);
        assert!(display.show_planet_panel);
        assert!(display.show_orbit_panel);
        assert!(!display.show_network_panel); // Off by default for single player
    }

    #[test]
    fn test_game_mode_changes_network_visibility() {
        let mut display = GameInfoDisplay::new();

        display.set_game_mode(GameMode::OnlineMultiplayer);
        assert!(display.show_network_panel);

        display.set_game_mode(GameMode::SinglePlayer);
        assert!(!display.show_network_panel);
    }

    #[test]
    fn test_toggle_panels() {
        let mut display = GameInfoDisplay::new();

        display.toggle_rocket_panel();
        assert!(!display.show_rocket_panel);

        display.toggle_rocket_panel();
        assert!(display.show_rocket_panel);
    }

    #[test]
    fn test_hide_all_panels() {
        let mut display = GameInfoDisplay::new();

        display.hide_all_panels();
        assert!(!display.show_rocket_panel);
        assert!(!display.show_planet_panel);
        assert!(!display.show_orbit_panel);
        assert!(!display.show_network_panel);
    }

    #[test]
    fn test_network_role_setting() {
        let mut display = GameInfoDisplay::new();

        display.set_network_role(NetworkRole::Host);
        assert_eq!(display.network_role, NetworkRole::Host);

        display.set_network_role(NetworkRole::Client);
        assert_eq!(display.network_role, NetworkRole::Client);
    }
}
