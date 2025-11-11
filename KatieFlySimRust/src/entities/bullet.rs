// Bullet - Projectile fired from rockets
// Affected by gravity, simple physics simulation

use macroquad::prelude::*;

use super::game_object::{GameObject, GameObjectData};

/// Bullet projectile with mass and lifetime
#[derive(Debug, Clone)]
pub struct Bullet {
    data: GameObjectData,
    mass: f32,
    lifetime: f32,        // Time since creation in seconds
    max_lifetime: f32,    // Despawn after this many seconds
    size: f32,            // Square size for rendering
}

impl Bullet {
    /// Create a new bullet
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        Bullet {
            data: GameObjectData::new(position, velocity, WHITE),
            mass: 1.0,  // 1 unit of mass
            lifetime: 0.0,
            max_lifetime: 360.0,  // Bullets last 360 seconds (6 minutes)
            size: 3.0,  // Small square, 3x3 pixels
        }
    }

    /// Get mass of bullet
    pub fn mass(&self) -> f32 {
        self.mass
    }

    /// Get lifetime
    pub fn lifetime(&self) -> f32 {
        self.lifetime
    }

    /// Check if bullet should be despawned
    pub fn should_despawn(&self) -> bool {
        self.lifetime >= self.max_lifetime
    }

    /// Get size for rendering
    pub fn size(&self) -> f32 {
        self.size
    }
}

impl GameObject for Bullet {
    fn update(&mut self, delta_time: f32) {
        // Update lifetime
        self.lifetime += delta_time;

        // Update position based on velocity
        self.data.position += self.data.velocity * delta_time;
    }

    fn draw(&self) {
        // Draw as small white square
        draw_rectangle(
            self.data.position.x - self.size / 2.0,
            self.data.position.y - self.size / 2.0,
            self.size,
            self.size,
            self.data.color,
        );

        // Draw outline for better visibility
        draw_rectangle_lines(
            self.data.position.x - self.size / 2.0,
            self.data.position.y - self.size / 2.0,
            self.size,
            self.size,
            1.0,
            Color::new(0.8, 0.8, 0.8, 0.8),
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
