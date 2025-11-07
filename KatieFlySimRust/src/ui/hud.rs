// HUD - Heads-up display for game information
// Shows rocket stats, speed, altitude, fuel, etc.

use macroquad::prelude::*;

use crate::entities::{GameObject, Rocket};

/// Heads-up display for showing game stats
pub struct Hud {
    position: Vec2,
    bg_size: Vec2,
    font_size: f32,
}

impl Hud {
    pub fn new(position: Vec2) -> Self {
        Hud {
            position,
            bg_size: Vec2::new(250.0, 175.0), // Increased from 150 to 175 for extra line
            font_size: 16.0,
        }
    }

    /// Draw HUD with rocket information
    pub fn draw_rocket_stats(&self, rocket: &Rocket, selected_thrust_level: f32) {
        // Draw background
        draw_rectangle(
            self.position.x,
            self.position.y,
            self.bg_size.x,
            self.bg_size.y,
            Color::new(0.0, 0.0, 0.0, 0.7),
        );

        // Draw border
        draw_rectangle_lines(
            self.position.x,
            self.position.y,
            self.bg_size.x,
            self.bg_size.y,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.4),
        );

        let mut y_offset = self.position.y + 10.0;
        let line_height = 25.0;

        // Velocity
        let velocity = rocket.velocity();
        let speed = (velocity.x * velocity.x + velocity.y * velocity.y).sqrt();
        self.draw_text(
            &format!("Speed: {:.1} m/s", speed),
            y_offset,
            GREEN,
        );
        y_offset += line_height;

        // Fuel
        let fuel_percent = rocket.fuel_percentage();
        let fuel_color = if fuel_percent > 50.0 {
            GREEN
        } else if fuel_percent > 20.0 {
            YELLOW
        } else {
            RED
        };
        self.draw_text(
            &format!("Fuel: {:.1}%", fuel_percent),
            y_offset,
            fuel_color,
        );
        y_offset += line_height;

        // Mass
        self.draw_text(
            &format!("Mass: {:.1} kg", rocket.mass()),
            y_offset,
            Color::new(0.0, 1.0, 1.0, 1.0), // CYAN
        );
        y_offset += line_height;

        // Selected thrust level (set by player with , and .)
        let selected_percent = selected_thrust_level * 100.0;
        self.draw_text(
            &format!("Thrust Set: {:.0}%", selected_percent),
            y_offset,
            Color::new(0.5, 0.8, 1.0, 1.0), // Light blue
        );
        y_offset += line_height;

        // Current thrust (actually being applied)
        let thrust_percent = rocket.thrust_level() * 100.0;
        self.draw_text(
            &format!("Thrust Now: {:.0}%", thrust_percent),
            y_offset,
            if thrust_percent > 0.0 {
                Color::new(1.0, 0.65, 0.0, 1.0) // Orange
            } else {
                WHITE
            },
        );
        y_offset += line_height;

        // Rotation with visual indicator
        let rotation_deg = rocket.rotation() * 180.0 / std::f32::consts::PI;
        self.draw_text(
            &format!("Heading: {:.0}°", rotation_deg),
            y_offset,
            WHITE,
        );

        // Draw visual rocket triangle indicator (fixed size, rotates with rocket)
        self.draw_heading_indicator(rocket.rotation(), y_offset);
    }

    /// Draw a visual rocket triangle showing the current heading
    fn draw_heading_indicator(&self, rotation: f32, y_offset: f32) {
        // Position to the right of the heading text
        let center_x = self.position.x + 190.0;
        let center_y = y_offset + self.font_size / 2.0;

        // Fixed size rocket triangle (independent of game zoom)
        let size = 20.0;

        // Define triangle points (pointing up in local space)
        // Tip of rocket
        let tip = Vec2::new(0.0, -size);
        // Left base
        let left = Vec2::new(-size * 0.4, size * 0.5);
        // Right base
        let right = Vec2::new(size * 0.4, size * 0.5);

        // Rotate points by rocket rotation
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();

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

        // Draw filled triangle
        draw_triangle(
            tip_screen,
            left_screen,
            right_screen,
            Color::new(0.3, 0.7, 1.0, 1.0), // Light blue
        );

        // Draw outline for better visibility
        draw_line(tip_screen.x, tip_screen.y, left_screen.x, left_screen.y, 2.0, WHITE);
        draw_line(left_screen.x, left_screen.y, right_screen.x, right_screen.y, 2.0, WHITE);
        draw_line(right_screen.x, right_screen.y, tip_screen.x, tip_screen.y, 2.0, WHITE);
    }

    /// Draw text at a specific y position
    fn draw_text(&self, text: &str, y: f32, color: Color) {
        draw_text(
            text,
            self.position.x + 10.0,
            y + self.font_size,
            self.font_size,
            color,
        );
    }

    /// Draw simple text overlay (for no active rocket)
    pub fn draw_message(&self, message: &str) {
        // Draw background
        draw_rectangle(
            self.position.x,
            self.position.y,
            self.bg_size.x,
            self.bg_size.y,
            Color::new(0.0, 0.0, 0.0, 0.7),
        );

        // Draw border
        draw_rectangle_lines(
            self.position.x,
            self.position.y,
            self.bg_size.x,
            self.bg_size.y,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.4),
        );

        self.draw_text(
            message,
            self.position.y + 65.0, // Center vertically
            WHITE,
        );
    }
}
