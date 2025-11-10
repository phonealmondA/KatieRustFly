// Game Info Display - Comprehensive information panels for all game modes
// Displays 5 information panels: Rocket, Planet, Orbit, Controls, Network

use macroquad::prelude::*;

use crate::entities::{Rocket, Planet};
use crate::systems::SatelliteNetworkStats;
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
    controls_panel: TextPanel,
    network_panel: TextPanel,

    // Panel visibility
    show_rocket_panel: bool,
    show_planet_panel: bool,
    show_orbit_panel: bool,
    show_controls_panel: bool,
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

        // Controls panel on the right side
        let screen_width = screen_width();
        let controls_panel = TextPanel::new(
            Vec2::new(screen_width - panel_width - panel_margin, panel_margin),
            Vec2::new(panel_width, 300.0),
        )
        .with_title("Controls")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(0.8, 0.8, 0.8, 0.6));

        // Network panel on the right side, below controls
        let network_panel = TextPanel::new(
            Vec2::new(screen_width - panel_width - panel_margin, 320.0),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Network")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(1.0, 0.5, 0.0, 0.6));

        GameInfoDisplay {
            rocket_panel,
            planet_panel,
            orbit_panel,
            controls_panel,
            network_panel,
            show_rocket_panel: true,
            show_planet_panel: true,
            show_orbit_panel: true,
            show_controls_panel: true,
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

        // Controls panel (hidden by default in split-screen)
        let controls_panel = TextPanel::new(
            Vec2::new(x_pos, 570.0),
            Vec2::new(panel_width, 150.0),
        )
        .with_title("Controls")
        .with_background_color(Color::new(0.0, 0.0, 0.0, 0.7))
        .with_border_color(Color::new(0.8, 0.8, 0.8, 0.6));

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
            controls_panel,
            network_panel,
            show_rocket_panel: true,
            show_planet_panel: true,
            show_orbit_panel: true,
            show_controls_panel: false,  // Hidden by default in split-screen
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

    pub fn toggle_controls_panel(&mut self) {
        self.show_controls_panel = !self.show_controls_panel;
    }

    pub fn toggle_network_panel(&mut self) {
        self.show_network_panel = !self.show_network_panel;
    }

    pub fn hide_all_panels(&mut self) {
        self.show_rocket_panel = false;
        self.show_planet_panel = false;
        self.show_orbit_panel = false;
        self.show_controls_panel = false;
        self.show_network_panel = false;
    }

    pub fn show_all_panels(&mut self) {
        self.show_rocket_panel = true;
        self.show_planet_panel = true;
        self.show_orbit_panel = true;
        self.show_controls_panel = true;
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
    pub fn update_planet_panel(&mut self, rocket_position: Vec2, planets: &[&Planet]) {
        let info = self.generate_planet_info(rocket_position, planets);
        self.planet_panel.set_text(&info);
    }

    /// Update orbital information panel
    pub fn update_orbit_panel(&mut self, rocket: &Rocket, primary_planet: Option<&Planet>) {
        let info = self.generate_orbit_info(rocket, primary_planet);
        self.orbit_panel.set_text(&info);
    }

    /// Update controls panel (static information)
    pub fn update_controls_panel(&mut self) {
        let controls = self.generate_controls_info();
        self.controls_panel.set_text(&controls);
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
        selected_thrust: f32,
        network_connected: bool,
        player_id: Option<usize>,
        player_count: usize,
        satellite_stats: Option<&SatelliteNetworkStats>,
    ) {
        if let Some(rocket) = rocket {
            let rocket_pos = rocket.position();
            self.update_rocket_panel(rocket, selected_thrust);
            self.update_planet_panel(rocket_pos, planets);

            // Find primary planet (nearest massive body)
            let primary_planet = Self::find_primary_planet(rocket_pos, planets);
            self.update_orbit_panel(rocket, primary_planet);
        }

        self.update_controls_panel();

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
    fn generate_planet_info(&self, rocket_position: Vec2, planets: &[&Planet]) -> String {
        let nearest = Self::find_nearest_planet(rocket_position, planets);

        if let Some(planet) = nearest {
            let distance = vector_helper::distance(rocket_position, planet.position());
            let mass = planet.mass();
            let radius = planet.radius();
            let velocity = planet.velocity().length();

            let in_fuel_range = distance <= planet.fuel_collection_range();
            let fuel_status = if in_fuel_range {
                format!("AVAILABLE (range: {:.0})", planet.fuel_collection_range())
            } else {
                format!("Out of range ({:.0} away)", distance - planet.fuel_collection_range())
            };

            format!(
                "Distance: {:.0} m\n\
                 Mass: {:.0} kg\n\
                 Radius: {:.0} m\n\
                 Velocity: {:.1} m/s\n\
                 Fuel: {}",
                distance, mass, radius, velocity, fuel_status
            )
        } else {
            "No planets detected".to_string()
        }
    }

    /// Generate orbital information text
    fn generate_orbit_info(&self, rocket: &Rocket, primary_planet: Option<&Planet>) -> String {
        if let Some(planet) = primary_planet {
            let distance = vector_helper::distance(rocket.position(), planet.position());
            let velocity = rocket.velocity().length();

            // Calculate orbital parameters
            let orbital_velocity = self.calculate_orbital_velocity(planet.mass(), distance);
            let escape_velocity = self.calculate_escape_velocity(planet.mass(), distance);

            let velocity_ratio = velocity / orbital_velocity;

            let orbit_status = if velocity < orbital_velocity * 0.9 {
                "DECAYING"
            } else if velocity > escape_velocity {
                "ESCAPING"
            } else if velocity > orbital_velocity * 1.1 {
                "CLIMBING"
            } else {
                "STABLE"
            };

            format!(
                "Altitude: {:.0} m\n\
                 Orbital V: {:.1} m/s\n\
                 Escape V: {:.1} m/s\n\
                 V Ratio: {:.2}x\n\
                 Status: {}",
                distance - planet.radius(),
                orbital_velocity,
                escape_velocity,
                velocity_ratio,
                orbit_status
            )
        } else {
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

    /// Generate controls information text
    fn generate_controls_info(&self) -> String {
        match self.game_mode {
            GameMode::SinglePlayer => {
                "Movement:\n\
                 SPACE - Thrust\n\
                 A/D or ← → - Rotate\n\
                 \n\
                 Actions:\n\
                 E - Launch/Detach\n\
                 C - Convert to Satellite\n\
                 F - Toggle Camera\n\
                 \n\
                 System:\n\
                 F5 - Quick Save\n\
                 ESC - Menu\n\
                 Mouse Wheel - Zoom"
                    .to_string()
            }
            GameMode::SplitScreen => {
                "Player 1:\n\
                 Arrow Keys - Move\n\
                 L - Launch\n\
                 T - Convert Satellite\n\
                 \n\
                 Player 2:\n\
                 WASD - Move\n\
                 K - Launch\n\
                 Y - Convert Satellite"
                    .to_string()
            }
            GameMode::OnlineMultiplayer => {
                "Controls:\n\
                 Same as Single Player\n\
                 \n\
                 Network:\n\
                 Your actions sync\n\
                 to other players\n\
                 \n\
                 Satellites visible\n\
                 to all players"
                    .to_string()
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

        // Right side panels
        self.controls_panel.set_position(Vec2::new(
            screen_width - self.panel_width - self.panel_margin,
            self.panel_margin,
        ));

        self.network_panel.set_position(Vec2::new(
            screen_width - self.panel_width - self.panel_margin,
            self.panel_margin + 310.0,
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

        if self.show_controls_panel {
            self.controls_panel.draw();
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
        assert!(display.show_controls_panel);
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
        assert!(!display.show_controls_panel);
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
