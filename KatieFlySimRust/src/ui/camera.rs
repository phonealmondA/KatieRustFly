// Camera - View management with zoom and pan
// Camera system for following entities and zooming

use macroquad::prelude::*;

/// Camera for managing the game view
pub struct Camera {
    camera: Camera2D,
    zoom_level: f32,
    target_zoom: f32,
    target_center: Vec2,
    zoom_speed: f32,
    follow_smoothing: f32,
    window_size: Vec2,
}

impl Camera {
    pub fn new(window_size: Vec2) -> Self {
        let center = Vec2::new(window_size.x / 2.0, window_size.y / 2.0);

        let camera = Camera2D {
            target: center,
            zoom: vec2(1.0 / window_size.x * 2.0, -1.0 / window_size.y * 2.0),
            offset: vec2(0.0, 0.0),
            rotation: 0.0,
            render_target: None,
            viewport: None,
        };

        Camera {
            camera,
            zoom_level: 1.0,
            target_zoom: 1.0,
            target_center: center,
            zoom_speed: 2.0,          // Reduced from 5.0 for slower zoom
            follow_smoothing: 0.5,    // Increased from 0.1 for tighter following
            window_size,
        }
    }

    /// Update camera (smooth zoom and follow)
    pub fn update(&mut self, delta_time: f32) {
        // Smooth zoom - use relative threshold since zoom can vary widely
        let threshold = self.target_zoom * 0.001; // 0.1% of target zoom
        if (self.zoom_level - self.target_zoom).abs() > threshold {
            let zoom_delta = (self.target_zoom - self.zoom_level) * self.zoom_speed * delta_time;
            self.zoom_level += zoom_delta;

            // Update camera zoom
            let zoom_scale = 1.0 / self.zoom_level;
            self.camera.zoom = vec2(
                zoom_scale / self.window_size.x * 2.0,
                -zoom_scale / self.window_size.y * 2.0,
            );
        }

        // Smooth follow
        let current_center = self.camera.target;
        let center_delta = self.target_center - current_center;

        if center_delta.x.abs() > 0.1 || center_delta.y.abs() > 0.1 {
            let smooth_delta = center_delta * self.follow_smoothing;
            self.camera.target = current_center + smooth_delta;
        }
    }

    /// Set target zoom level
    pub fn set_target_zoom(&mut self, zoom: f32) {
        self.target_zoom = zoom.max(0.1).min(2000000.0); // Clamp zoom (0.1 = very close, 2000000.0 = entire solar system)
    }

    /// Adjust zoom by a delta
    pub fn adjust_zoom(&mut self, delta: f32) {
        self.set_target_zoom(self.target_zoom + delta);
    }

    /// Set center position (instant)
    pub fn set_center(&mut self, center: Vec2) {
        self.target_center = center;
        self.camera.target = center;
    }

    /// Set target center (smooth follow)
    pub fn set_target_center(&mut self, center: Vec2) {
        self.target_center = center;
    }

    /// Follow an entity position
    pub fn follow(&mut self, position: Vec2) {
        self.set_target_center(position);
    }

    /// Get the current camera (for use with set_camera())
    pub fn camera(&self) -> &Camera2D {
        &self.camera
    }

    /// Get zoom level
    pub fn zoom_level(&self) -> f32 {
        self.zoom_level
    }

    /// Reset camera to default
    pub fn reset(&mut self, window_size: Vec2) {
        self.zoom_level = 1.0;
        self.target_zoom = 1.0;
        let center = Vec2::new(window_size.x / 2.0, window_size.y / 2.0);
        self.target_center = center;
        self.camera.target = center;
        self.window_size = window_size;

        let zoom_scale = 1.0 / self.zoom_level;
        self.camera.zoom = vec2(
            zoom_scale / window_size.x * 2.0,
            -zoom_scale / window_size.y * 2.0,
        );
        self.camera.render_target = None;
        self.camera.viewport = None;
    }

    /// Handle window resize
    pub fn handle_resize(&mut self, new_size: Vec2) {
        self.window_size = new_size;
        let zoom_scale = 1.0 / self.zoom_level;
        self.camera.zoom = vec2(
            zoom_scale / new_size.x * 2.0,
            -zoom_scale / new_size.y * 2.0,
        );
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Use macroquad's camera to convert screen to world coordinates
        self.camera.screen_to_world(screen_pos)
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        // Use macroquad's camera to convert world to screen coordinates
        self.camera.world_to_screen(world_pos)
    }

    /// Set camera position (instant, no smoothing)
    pub fn set_position(&mut self, position: Vec2) {
        self.set_center(position);
    }

    /// Get current zoom level
    pub fn zoom(&self) -> f32 {
        self.zoom_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new(Vec2::new(1920.0, 1080.0));
        assert_eq!(camera.zoom_level(), 1.0);
    }

    #[test]
    fn test_zoom_clamping() {
        let mut camera = Camera::new(Vec2::new(1920.0, 1080.0));

        camera.set_target_zoom(3000000.0); // Too high (too zoomed out)
        assert_eq!(camera.target_zoom, 2000000.0); // Max is 2,000,000

        camera.set_target_zoom(0.01); // Too low (too zoomed in)
        assert_eq!(camera.target_zoom, 0.1); // Minimum is 0.1
    }

    #[test]
    fn test_follow() {
        let mut camera = Camera::new(Vec2::new(1920.0, 1080.0));
        let target_pos = Vec2::new(500.0, 300.0);

        camera.follow(target_pos);
        assert_eq!(camera.target_center, target_pos);
    }
}
