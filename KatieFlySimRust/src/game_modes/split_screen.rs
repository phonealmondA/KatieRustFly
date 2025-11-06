// Split Screen Mode - Local multiplayer with viewports (placeholder)
// Phase 13: Split Screen

use macroquad::prelude::*;
use crate::systems::World;
use crate::ui::Camera;

/// Split screen viewport
pub struct Viewport {
    camera: Camera,
    rect: Rect,
    player_index: usize,
}

impl Viewport {
    pub fn new(rect: Rect, player_index: usize) -> Self {
        Viewport {
            camera: Camera::new(Vec2::new(rect.w, rect.h)),
            rect,
            player_index,
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &self.camera
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn player_index(&self) -> usize {
        self.player_index
    }
}

/// Split Screen Manager (placeholder)
pub struct SplitScreenMode {
    world: World,
    viewports: Vec<Viewport>,
    running: bool,
}

impl SplitScreenMode {
    pub fn new() -> Self {
        let screen_width = screen_width();
        let screen_height = screen_height();

        // Create two viewports (top/bottom split)
        let viewports = vec![
            Viewport::new(
                Rect::new(0.0, 0.0, screen_width, screen_height / 2.0),
                0,
            ),
            Viewport::new(
                Rect::new(0.0, screen_height / 2.0, screen_width, screen_height / 2.0),
                1,
            ),
        ];

        SplitScreenMode {
            world: World::new(),
            viewports,
            running: false,
        }
    }

    /// Initialize split screen game
    pub fn init(&mut self) {
        // TODO: Initialize game world with multiple players
        self.running = true;
    }

    /// Update split screen game
    pub fn update(&mut self, _delta_time: f32) {
        // TODO: Update world for all players
        if self.running {
            self.world.update(_delta_time);
        }
    }

    /// Handle input for player
    pub fn handle_input(&mut self, _player_index: usize) {
        // TODO: Handle input for specific player
        // Player 1: WASD + Space
        // Player 2: Arrow Keys + Enter
    }

    /// Render split screen
    pub fn render(&self) {
        clear_background(BLACK);

        for viewport in &self.viewports {
            // Set up viewport rendering
            // TODO: Render game world from this camera's perspective

            // Draw separator line between viewports
            if viewport.player_index == 0 {
                let y = viewport.rect.y + viewport.rect.h;
                draw_line(
                    0.0,
                    y,
                    screen_width(),
                    y,
                    3.0,
                    WHITE,
                );
            }

            // Draw player indicator
            let label = format!("Player {}", viewport.player_index + 1);
            draw_text(
                &label,
                viewport.rect.x + 10.0,
                viewport.rect.y + 30.0,
                24.0,
                YELLOW,
            );
        }

        // Draw "not implemented" message
        let message = "Split Screen Mode - In Development";
        let text_dims = measure_text(message, None, 32, 1.0);
        draw_text(
            message,
            screen_width() / 2.0 - text_dims.width / 2.0,
            screen_height() / 2.0,
            32.0,
            RED,
        );
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for SplitScreenMode {
    fn default() -> Self {
        Self::new()
    }
}
