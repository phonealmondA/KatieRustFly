// Split Screen Mode - Local multiplayer with viewports
// Phase 13: Split Screen

use macroquad::prelude::*;
use crate::systems::World;
use crate::ui::Camera;
use crate::entities::Rocket;

/// Input mapping for each player
#[derive(Debug, Clone, Copy)]
pub struct PlayerInputMapping {
    pub thrust: KeyCode,
    pub rotate_left: KeyCode,
    pub rotate_right: KeyCode,
    pub launch: KeyCode,
    pub convert_to_satellite: KeyCode,
}

impl PlayerInputMapping {
    /// Player 1 controls (WASD + Space)
    pub fn player1() -> Self {
        PlayerInputMapping {
            thrust: KeyCode::W,
            rotate_left: KeyCode::A,
            rotate_right: KeyCode::D,
            launch: KeyCode::Space,
            convert_to_satellite: KeyCode::S,
        }
    }

    /// Player 2 controls (Arrow Keys + Enter)
    pub fn player2() -> Self {
        PlayerInputMapping {
            thrust: KeyCode::Up,
            rotate_left: KeyCode::Left,
            rotate_right: KeyCode::Right,
            launch: KeyCode::Enter,
            convert_to_satellite: KeyCode::Down,
        }
    }

    /// Check if thrust key is pressed
    pub fn is_thrust_pressed(&self) -> bool {
        is_key_down(self.thrust)
    }

    /// Check if rotate left key is pressed
    pub fn is_rotate_left_pressed(&self) -> bool {
        is_key_down(self.rotate_left)
    }

    /// Check if rotate right key is pressed
    pub fn is_rotate_right_pressed(&self) -> bool {
        is_key_down(self.rotate_right)
    }

    /// Check if launch key is just pressed
    pub fn is_launch_pressed(&self) -> bool {
        is_key_pressed(self.launch)
    }

    /// Check if convert to satellite key is just pressed
    pub fn is_convert_pressed(&self) -> bool {
        is_key_pressed(self.convert_to_satellite)
    }
}

/// Split screen viewport
pub struct Viewport {
    camera: Camera,
    rect: Rect,
    player_index: usize,
    rocket_id: Option<usize>,
    input_mapping: PlayerInputMapping,
}

impl Viewport {
    pub fn new(rect: Rect, player_index: usize) -> Self {
        let input_mapping = if player_index == 0 {
            PlayerInputMapping::player1()
        } else {
            PlayerInputMapping::player2()
        };

        Viewport {
            camera: Camera::new(Vec2::new(rect.w, rect.h)),
            rect,
            player_index,
            rocket_id: None,
            input_mapping,
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn player_index(&self) -> usize {
        self.player_index
    }

    pub fn set_rocket_id(&mut self, rocket_id: Option<usize>) {
        self.rocket_id = rocket_id;
    }

    pub fn rocket_id(&self) -> Option<usize> {
        self.rocket_id
    }

    pub fn input_mapping(&self) -> &PlayerInputMapping {
        &self.input_mapping
    }

    /// Update viewport camera to follow rocket
    pub fn update_camera(&mut self, world: &World, delta_time: f32) {
        if let Some(rocket_id) = self.rocket_id {
            if let Some(rocket) = world.get_rocket(rocket_id) {
                self.camera.follow(rocket.position());
            }
        }

        self.camera.update(delta_time);
    }
}

/// Split Screen Manager
pub struct SplitScreenMode {
    world: World,
    viewports: Vec<Viewport>,
    running: bool,
    player_count: usize,
}

impl SplitScreenMode {
    pub fn new(player_count: usize) -> Self {
        let scr_width = screen_width();
        let scr_height = screen_height();

        // Create viewports based on player count
        let viewports = if player_count == 2 {
            // Horizontal split (top/bottom)
            vec![
                Viewport::new(
                    Rect::new(0.0, 0.0, scr_width, scr_height / 2.0),
                    0,
                ),
                Viewport::new(
                    Rect::new(0.0, scr_height / 2.0, scr_width, scr_height / 2.0),
                    1,
                ),
            ]
        } else if player_count == 3 {
            // Top half split, bottom full
            vec![
                Viewport::new(
                    Rect::new(0.0, 0.0, scr_width / 2.0, scr_height / 2.0),
                    0,
                ),
                Viewport::new(
                    Rect::new(scr_width / 2.0, 0.0, scr_width / 2.0, scr_height / 2.0),
                    1,
                ),
                Viewport::new(
                    Rect::new(0.0, scr_height / 2.0, scr_width, scr_height / 2.0),
                    2,
                ),
            ]
        } else if player_count == 4 {
            // 2x2 grid
            vec![
                Viewport::new(
                    Rect::new(0.0, 0.0, scr_width / 2.0, scr_height / 2.0),
                    0,
                ),
                Viewport::new(
                    Rect::new(scr_width / 2.0, 0.0, scr_width / 2.0, scr_height / 2.0),
                    1,
                ),
                Viewport::new(
                    Rect::new(0.0, scr_height / 2.0, scr_width / 2.0, scr_height / 2.0),
                    2,
                ),
                Viewport::new(
                    Rect::new(scr_width / 2.0, scr_height / 2.0, scr_width / 2.0, scr_height / 2.0),
                    3,
                ),
            ]
        } else {
            // Default: single player (full screen)
            vec![
                Viewport::new(
                    Rect::new(0.0, 0.0, scr_width, scr_height),
                    0,
                ),
            ]
        };

        SplitScreenMode {
            world: World::new(),
            viewports,
            running: false,
            player_count,
        }
    }

    /// Initialize split screen game
    pub fn init(&mut self) {
        self.running = true;

        // Spawn rockets for each player at different locations
        let spawn_positions = [
            Vec2::new(100.0, 100.0),
            Vec2::new(-100.0, -100.0),
            Vec2::new(100.0, -100.0),
            Vec2::new(-100.0, 100.0),
        ];

        for (i, viewport) in self.viewports.iter_mut().enumerate() {
            if i < spawn_positions.len() {
                let rocket_id = self.world.spawn_rocket_at(
                    spawn_positions[i],
                    Vec2::ZERO,
                    0.0,
                );
                viewport.set_rocket_id(Some(rocket_id));

                // Initialize camera position
                viewport.camera_mut().set_position(spawn_positions[i]);
            }
        }
    }

    /// Update split screen game
    pub fn update(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }

        // Update world physics
        self.world.update(delta_time);

        // Update each viewport
        for viewport in &mut self.viewports {
            viewport.update_camera(&self.world, delta_time);
        }
    }

    /// Handle input for all players
    pub fn handle_input(&mut self) {
        for viewport in &mut self.viewports {
            let mapping = viewport.input_mapping();

            if let Some(rocket_id) = viewport.rocket_id() {
                // Apply thrust
                if mapping.is_thrust_pressed() {
                    self.world.set_rocket_thrust(rocket_id, true);
                } else {
                    self.world.set_rocket_thrust(rocket_id, false);
                }

                // Apply rotation
                if mapping.is_rotate_left_pressed() {
                    self.world.rotate_rocket(rocket_id, 3.0);
                } else if mapping.is_rotate_right_pressed() {
                    self.world.rotate_rocket(rocket_id, -3.0);
                }

                // Handle launch (separate rocket into stages)
                if mapping.is_launch_pressed() {
                    // TODO: Implement stage separation
                }

                // Handle convert to satellite
                if mapping.is_convert_pressed() {
                    self.world.convert_rocket_to_satellite(rocket_id);
                    viewport.set_rocket_id(None); // Rocket is now a satellite
                }
            }
        }

        // Check for escape to return to menu
        if is_key_pressed(KeyCode::Escape) {
            self.running = false;
        }
    }

    /// Render split screen
    pub fn render(&self) {
        clear_background(BLACK);

        for (i, viewport) in self.viewports.iter().enumerate() {
            // Set scissor rectangle for this viewport
            gl_use_default_material();

            // Render game world for this viewport
            self.render_viewport(viewport);

            // Draw viewport borders
            if i < self.viewports.len() - 1 {
                // Draw separator lines
                if self.player_count == 2 {
                    // Horizontal line
                    let y = viewport.rect.y + viewport.rect.h;
                    draw_line(
                        0.0,
                        y,
                        screen_width(),
                        y,
                        3.0,
                        WHITE,
                    );
                } else if self.player_count >= 3 {
                    // Grid lines
                    let vp_rect = viewport.rect;

                    // Vertical line
                    if vp_rect.x == 0.0 && vp_rect.w < screen_width() {
                        draw_line(
                            vp_rect.x + vp_rect.w,
                            vp_rect.y,
                            vp_rect.x + vp_rect.w,
                            vp_rect.y + vp_rect.h,
                            3.0,
                            WHITE,
                        );
                    }

                    // Horizontal line
                    if vp_rect.y == 0.0 && vp_rect.h < screen_height() {
                        draw_line(
                            vp_rect.x,
                            vp_rect.y + vp_rect.h,
                            vp_rect.x + vp_rect.w,
                            vp_rect.y + vp_rect.h,
                            3.0,
                            WHITE,
                        );
                    }
                }
            }

            // Draw player indicator
            let label = format!("P{}", viewport.player_index + 1);
            let color = match viewport.player_index {
                0 => YELLOW,
                1 => GREEN,
                2 => BLUE,
                _ => RED,
            };

            draw_text(
                &label,
                viewport.rect.x + 10.0,
                viewport.rect.y + 25.0,
                20.0,
                color,
            );

            // Draw rocket info if alive
            if let Some(rocket_id) = viewport.rocket_id() {
                if let Some(rocket) = self.world.get_rocket(rocket_id) {
                    let fuel_text = format!("Fuel: {:.0}", rocket.current_fuel());
                    draw_text(
                        &fuel_text,
                        viewport.rect.x + 10.0,
                        viewport.rect.y + 45.0,
                        16.0,
                        WHITE,
                    );
                }
            }
        }

        // Draw controls reminder at bottom
        let controls = "P1: WASD+Space  |  P2: Arrows+Enter  |  ESC: Menu";
        let text_dims = measure_text(controls, None, 16, 1.0);
        draw_text(
            controls,
            screen_width() / 2.0 - text_dims.width / 2.0,
            screen_height() - 10.0,
            16.0,
            GRAY,
        );
    }

    /// Render a single viewport
    fn render_viewport(&self, viewport: &Viewport) {
        let vp_rect = viewport.rect;
        let camera = viewport.camera();

        // Calculate world bounds visible in this viewport
        let world_min = camera.screen_to_world(Vec2::new(vp_rect.x, vp_rect.y));
        let world_max = camera.screen_to_world(Vec2::new(vp_rect.x + vp_rect.w, vp_rect.y + vp_rect.h));

        // Render planets
        for planet in self.world.planets() {
            let screen_pos = camera.world_to_screen(planet.position());

            // Check if within viewport
            if screen_pos.x >= vp_rect.x && screen_pos.x <= vp_rect.x + vp_rect.w &&
               screen_pos.y >= vp_rect.y && screen_pos.y <= vp_rect.y + vp_rect.h {
                draw_circle(
                    screen_pos.x,
                    screen_pos.y,
                    planet.radius() * camera.zoom(),
                    BLUE,
                );
            }
        }

        // Render rockets
        for rocket in self.world.rockets() {
            let screen_pos = camera.world_to_screen(rocket.position());

            // Check if within viewport
            if screen_pos.x >= vp_rect.x && screen_pos.x <= vp_rect.x + vp_rect.w &&
               screen_pos.y >= vp_rect.y && screen_pos.y <= vp_rect.y + vp_rect.h {
                // Draw rocket triangle
                let size = 10.0;
                let rotation = rocket.rotation();

                // Calculate triangle points
                let p1 = Vec2::new(
                    screen_pos.x + size * rotation.cos(),
                    screen_pos.y + size * rotation.sin(),
                );
                let p2 = Vec2::new(
                    screen_pos.x + size * (rotation + 2.4).cos(),
                    screen_pos.y + size * (rotation + 2.4).sin(),
                );
                let p3 = Vec2::new(
                    screen_pos.x + size * (rotation - 2.4).cos(),
                    screen_pos.y + size * (rotation - 2.4).sin(),
                );

                draw_triangle(p1, p2, p3, RED);

                // Draw thrust flame if active
                if rocket.thrust_level() > 0.0 {
                    let flame_len = 15.0;
                    let flame_start = Vec2::new(
                        screen_pos.x - (size * 0.5) * rotation.cos(),
                        screen_pos.y - (size * 0.5) * rotation.sin(),
                    );
                    let flame_end = Vec2::new(
                        flame_start.x - flame_len * rotation.cos(),
                        flame_start.y - flame_len * rotation.sin(),
                    );

                    draw_line(
                        flame_start.x,
                        flame_start.y,
                        flame_end.x,
                        flame_end.y,
                        4.0,
                        ORANGE,
                    );
                }
            }
        }

        // Render satellites
        for satellite in self.world.satellites() {
            let screen_pos = camera.world_to_screen(satellite.position());

            // Check if within viewport
            if screen_pos.x >= vp_rect.x && screen_pos.x <= vp_rect.x + vp_rect.w &&
               screen_pos.y >= vp_rect.y && screen_pos.y <= vp_rect.y + vp_rect.h {
                draw_circle(
                    screen_pos.x,
                    screen_pos.y,
                    5.0,
                    GREEN,
                );

                // Draw solar panels
                let panel_size = 8.0;
                draw_rectangle(
                    screen_pos.x - panel_size - 5.0,
                    screen_pos.y - 2.0,
                    panel_size,
                    4.0,
                    YELLOW,
                );
                draw_rectangle(
                    screen_pos.x + 5.0,
                    screen_pos.y - 2.0,
                    panel_size,
                    4.0,
                    YELLOW,
                );
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn viewports(&self) -> &[Viewport] {
        &self.viewports
    }
}

impl Default for SplitScreenMode {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_input_mapping() {
        let p1 = PlayerInputMapping::player1();
        assert_eq!(p1.thrust, KeyCode::W);
        assert_eq!(p1.rotate_left, KeyCode::A);

        let p2 = PlayerInputMapping::player2();
        assert_eq!(p2.thrust, KeyCode::Up);
        assert_eq!(p2.rotate_left, KeyCode::Left);
    }

    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport::new(Rect::new(0.0, 0.0, 400.0, 300.0), 0);
        assert_eq!(viewport.player_index(), 0);
        assert_eq!(viewport.rocket_id(), None);
    }

    // Note: Tests for SplitScreenMode::new() are skipped because they require
    // a Macroquad window context (screen_width/screen_height calls)
}
