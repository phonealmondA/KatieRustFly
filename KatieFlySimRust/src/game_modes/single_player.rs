// Single Player Game - Main single player game mode
// Integrates all systems for playable game

use macroquad::prelude::*;

use crate::entities::{GameObject, Planet, Rocket};
use crate::game_constants::GameConstants;
use crate::physics::{TrajectoryPoint, TrajectoryPredictor};
use crate::save_system::{GameSaveData, SavedCamera};
use crate::systems::World;
use crate::ui::{Camera, Hud};

/// Single player game result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerResult {
    Continue,
    ReturnToMenu,
    Quit,
}

/// Single player game mode
pub struct SinglePlayerGame {
    world: World,
    camera: Camera,
    hud: Hud,
    trajectory_predictor: TrajectoryPredictor,
    game_time: f32,
    is_paused: bool,
    show_controls: bool,

    // Input state
    selected_thrust_level: f32, // 0.0 to 1.0 (0% to 100%)
    rotation_input: f32,
    idle_timer: f32, // Time since last input (for auto-expanding trajectory)

    // Milestone-based trajectory system (MUCH simpler and more efficient)
    milestone_nodes: Vec<TrajectoryPoint>, // 10 milestone nodes, 300 steps (150s) apart
    milestone_step_size: usize, // Steps between each milestone (300 = 150 seconds)
    current_milestone_index: usize, // Which milestone rocket is currently traveling toward
    trajectory_orbit_detected: bool, // True when trajectory loops back to start

    // Save/load
    current_save_name: Option<String>,
    last_auto_save: f32,
    auto_save_interval: f32,
}

impl SinglePlayerGame {
    pub fn new(window_size: Vec2) -> Self {
        SinglePlayerGame {
            world: World::new(),
            camera: Camera::new(window_size),
            hud: Hud::new(Vec2::new(10.0, 10.0)),
            trajectory_predictor: TrajectoryPredictor::new(),
            game_time: 0.0,
            is_paused: false,
            show_controls: false,
            selected_thrust_level: 0.0, // Start at 0% thrust
            rotation_input: 0.0,
            idle_timer: 0.0, // Start at 0
            milestone_nodes: Vec::new(),
            milestone_step_size: 300, // 300 steps = 150 seconds between milestones
            current_milestone_index: 0,
            trajectory_orbit_detected: false,
            current_save_name: None,
            last_auto_save: 0.0,
            auto_save_interval: 60.0, // Auto-save every 60 seconds
        }
    }

    /// Initialize a new game with default setup
    pub fn initialize_new_game(&mut self) {
        self.world.clear_all();
        self.game_time = 0.0;

        // Create main planet (like Earth)
        let main_planet = Planet::new(
            Vec2::new(GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y),
            GameConstants::MAIN_PLANET_RADIUS,
            GameConstants::MAIN_PLANET_MASS,
            BLUE,
        );
        log::info!("Main planet: pos=({}, {}), radius={}, mass={}",
            GameConstants::MAIN_PLANET_X, GameConstants::MAIN_PLANET_Y,
            GameConstants::MAIN_PLANET_RADIUS, GameConstants::MAIN_PLANET_MASS);
        self.world.add_planet(main_planet);

        // Create secondary planet (like Moon)
        let moon_x = *crate::game_constants::SECONDARY_PLANET_X;
        let moon_y = *crate::game_constants::SECONDARY_PLANET_Y;
        let moon_radius = GameConstants::SECONDARY_PLANET_RADIUS;
        let moon_velocity = *crate::game_constants::SECONDARY_PLANET_ORBITAL_VELOCITY;

        log::info!("Moon: pos=({}, {}), radius={}, mass={}, velocity={}",
            moon_x, moon_y, moon_radius, GameConstants::SECONDARY_PLANET_MASS, moon_velocity);
        log::info!("Distance from main planet: {}",
            ((moon_x - GameConstants::MAIN_PLANET_X).powi(2) + (moon_y - GameConstants::MAIN_PLANET_Y).powi(2)).sqrt());

        let mut secondary_planet = Planet::new(
            Vec2::new(moon_x, moon_y),
            moon_radius,
            GameConstants::SECONDARY_PLANET_MASS,
            Color::from_rgba(150, 150, 150, 255),
        );

        // Set orbital velocity for secondary planet
        secondary_planet.set_velocity(Vec2::new(
            0.0,
            -moon_velocity,
        ));

        self.world.add_planet(secondary_planet);

        // Create starting rocket near main planet
        let rocket_spawn_distance = GameConstants::MAIN_PLANET_RADIUS + 200.0;
        let rocket = Rocket::new(
            Vec2::new(
                GameConstants::MAIN_PLANET_X + rocket_spawn_distance,
                GameConstants::MAIN_PLANET_Y,
            ),
            Vec2::new(0.0, 0.0),
            WHITE,
            GameConstants::ROCKET_BASE_MASS,
        );

        let rocket_id = self.world.add_rocket(rocket);

        // Set camera to follow rocket
        if let Some(rocket) = self.world.get_rocket(rocket_id) {
            self.camera.set_center(rocket.position());
        }

        log::info!("New game initialized");
    }

    /// Load game from save data
    pub fn load_from_save(&mut self, save_data: GameSaveData, save_name: String) {
        self.world.clear_all();
        self.game_time = save_data.game_time;

        // Load planets
        for saved_planet in save_data.planets {
            let (_id, planet) = saved_planet.to_planet();
            self.world.add_planet(planet);
        }

        // Load rockets
        for saved_rocket in save_data.rockets {
            let (_id, rocket) = saved_rocket.to_rocket();
            self.world.add_rocket(rocket);
        }

        // Load satellites
        for saved_satellite in save_data.satellites {
            let (_id, satellite) = saved_satellite.to_satellite();
            self.world.add_satellite(satellite);
        }

        // Set active rocket
        self.world.set_active_rocket(save_data.active_rocket_id);

        // Restore camera
        self.camera.set_center(save_data.camera.center.into());
        self.camera.set_target_zoom(save_data.camera.zoom);

        self.current_save_name = Some(save_name.clone());

        log::info!("Game loaded from save: {}", save_name);
    }

    /// Save current game state
    pub fn save_game(&self, save_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let save_data = self.create_save_data();
        save_data.save_to_file(save_name)?;
        log::info!("Game saved: {}", save_name);
        Ok(())
    }

    /// Create save data from current game state
    fn create_save_data(&self) -> GameSaveData {
        let mut save_data = GameSaveData::new();
        save_data.game_time = self.game_time;

        // Save planets (we need to iterate with IDs - simplified for now)
        // In a real implementation, World would provide an iterator with IDs

        // Save camera
        save_data.camera = SavedCamera {
            center: self.camera.camera().target.into(),
            zoom: self.camera.zoom_level(),
        };

        save_data.active_rocket_id = self.world.active_rocket_id();

        save_data
    }

    /// Handle input for game controls
    pub fn handle_input(&mut self) -> SinglePlayerResult {
        // Check for escape to return to menu or close controls popup
        if is_key_pressed(KeyCode::Escape) {
            if self.show_controls {
                self.show_controls = false;
                self.is_paused = false;
            } else {
                return SinglePlayerResult::ReturnToMenu;
            }
        }

        // Toggle controls menu with Enter key
        if is_key_pressed(KeyCode::Enter) {
            self.show_controls = !self.show_controls;
            self.is_paused = self.show_controls; // Pause when showing controls
        }

        // Handle mouse click for controls button and popup
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let screen_w = screen_width();

            // Controls button in top-right corner (40x30 button with 10px margin)
            let button_x = screen_w - 50.0;
            let button_y = 10.0;
            let button_w = 40.0;
            let button_h = 30.0;

            // Check if click is on the button first
            let clicked_button = mouse_pos.0 >= button_x && mouse_pos.0 <= button_x + button_w &&
                                 mouse_pos.1 >= button_y && mouse_pos.1 <= button_y + button_h;

            if clicked_button {
                self.show_controls = !self.show_controls;
                self.is_paused = self.show_controls; // Pause when showing controls
                log::info!("Controls button clicked, show_controls: {}", self.show_controls);
            } else if self.show_controls {
                // Only check "click outside to close" if we didn't click the button
                let popup_x = screen_w / 2.0 - 200.0;
                let popup_y = screen_height() / 2.0 - 250.0;
                let popup_w = 400.0;
                let popup_h = 500.0;

                // Check if click is inside the popup
                let clicked_inside = mouse_pos.0 >= popup_x && mouse_pos.0 <= popup_x + popup_w &&
                                     mouse_pos.1 >= popup_y && mouse_pos.1 <= popup_y + popup_h;

                // Close if clicking outside the popup
                if !clicked_inside {
                    self.show_controls = false;
                    self.is_paused = false;
                    log::info!("Clicked outside popup, closing controls");
                }
            }
        }

        // Toggle pause (only if controls not showing)
        if is_key_pressed(KeyCode::P) && !self.show_controls {
            self.is_paused = !self.is_paused;
        }

        // Quick save
        if is_key_pressed(KeyCode::F5) {
            if let Err(e) = self.save_game("quicksave") {
                log::error!("Failed to quick save: {}", e);
            }
        }

        // Mouse wheel zoom (reduced delta for finer control)
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel != 0.0 {
            self.camera.adjust_zoom(-mouse_wheel * 0.02);
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }

        // Keyboard zoom controls (E = zoom out, Q = zoom in)
        // Note: zoom_scale = 1/zoom_level, so larger zoom_level = more zoomed out
        if is_key_down(KeyCode::Q) {
            self.camera.adjust_zoom(-0.02); // Gradual zoom in (decrease zoom_level)
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }
        if is_key_down(KeyCode::E) {
            self.camera.adjust_zoom(0.02); // Gradual zoom out (increase zoom_level)
            self.idle_timer = 0.0; // Reset idle timer on zoom
            self.reset_trajectory_cache(); // Clear cache on zoom
        }

        SinglePlayerResult::Continue
    }

    /// Update game state
    pub fn update(&mut self, delta_time: f32) {
        if self.is_paused {
            return;
        }

        self.game_time += delta_time;

        // Increment idle timer
        self.idle_timer += delta_time;

        // Handle input for active rocket
        self.update_rocket_input();

        // Update world (physics, entities)
        self.world.update(delta_time);

        // Update camera to follow active rocket
        if let Some(rocket) = self.world.get_active_rocket() {
            self.camera.follow(rocket.position());
        }

        self.camera.update(delta_time);

        // Auto-save
        if self.game_time - self.last_auto_save > self.auto_save_interval {
            if let Err(e) = self.save_game("autosave") {
                log::error!("Auto-save failed: {}", e);
            }
            self.last_auto_save = self.game_time;
        }
    }

    /// Update rocket based on keyboard input
    fn update_rocket_input(&mut self) {
        // Check if any input is detected and reset idle timer
        let has_input = is_key_pressed(KeyCode::Comma) || is_key_pressed(KeyCode::Period) ||
                        is_key_down(KeyCode::Space) ||
                        is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) ||
                        is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) ||
                        is_key_pressed(KeyCode::L) || is_key_pressed(KeyCode::T);

        if has_input {
            self.idle_timer = 0.0;
            self.reset_trajectory_cache(); // Clear cache on input
        }

        // Thrust level adjustment (comma to decrease, period to increase)
        if is_key_pressed(KeyCode::Comma) {
            self.selected_thrust_level = (self.selected_thrust_level - 0.05).max(0.0);
            log::info!("Thrust level decreased to {}%", (self.selected_thrust_level * 100.0) as i32);
        }
        if is_key_pressed(KeyCode::Period) {
            self.selected_thrust_level = (self.selected_thrust_level + 0.05).min(1.0);
            log::info!("Thrust level increased to {}%", (self.selected_thrust_level * 100.0) as i32);
        }

        // Get input state
        let mut thrust_level = 0.0;
        let mut rotation_delta = 0.0;

        // Thrust controls - space bar applies the selected thrust level
        if is_key_down(KeyCode::Space) {
            thrust_level = self.selected_thrust_level;
        }

        // Rotation controls
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            rotation_delta = 3.0; // degrees per frame
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            rotation_delta = -3.0;
        }

        // Convert degrees to radians
        let rotation_radians = rotation_delta * std::f32::consts::PI / 180.0;

        // Apply to active rocket
        if let Some(rocket) = self.world.get_active_rocket_mut() {
            rocket.set_thrust_level(thrust_level);
            if rotation_delta != 0.0 {
                rocket.rotate(rotation_radians);
            }
        }

        // Launch new rocket (L key)
        if is_key_pressed(KeyCode::L) {
            self.launch_new_rocket();
        }

        // Convert to satellite (T key)
        if is_key_pressed(KeyCode::T) {
            if let Some(rocket_id) = self.world.active_rocket_id() {
                if self.world.convert_rocket_to_satellite(rocket_id).is_some() {
                    log::info!("Rocket converted to satellite");
                }
            }
        }
    }

    /// Launch a new rocket from the active rocket's position
    fn launch_new_rocket(&mut self) {
        if let Some(current_rocket) = self.world.get_active_rocket() {
            let new_rocket = Rocket::new(
                current_rocket.position(),
                current_rocket.velocity(),
                Color::from_rgba(200, 200, 255, 255),
                GameConstants::ROCKET_BASE_MASS,
            );

            let new_id = self.world.add_rocket(new_rocket);
            self.world.set_active_rocket(Some(new_id));

            log::info!("New rocket launched");
        }
    }

    /// Render the game
    pub fn render(&mut self) {
        // Set camera view
        set_camera(self.camera.camera());

        // Render world
        self.world.render();

        // Get all planets for trajectory calculations
        let all_planets: Vec<&Planet> = self.world.planets().collect();

        // Get current zoom level for scaling trajectory line thickness
        let zoom_level = self.camera.zoom_level();

        // Draw moon trajectory (if moon exists)
        // Identify Earth (large planet) and Moon (small planet) by radius
        // HashMap iteration order is unpredictable, so we can't rely on index
        if all_planets.len() >= 2 {
            // Earth is the planet with the larger radius, Moon is the smaller one
            let (earth, moon) = if all_planets[0].radius() > all_planets[1].radius() {
                (all_planets[0], all_planets[1])
            } else {
                (all_planets[1], all_planets[0])
            };

            log::info!("Moon position: ({}, {}), velocity: ({}, {}), radius: {}",
                moon.position().x, moon.position().y,
                moon.velocity().x, moon.velocity().y, moon.radius());
            log::info!("Earth position: ({}, {}), radius: {}",
                earth.position().x, earth.position().y, earth.radius());

            let earth_only: Vec<&Planet> = vec![earth];

            let (moon_trajectory, moon_orbit_closes) = self.trajectory_predictor.predict_planet_trajectory(
                moon,
                &earth_only,
                0.5,
                840, // 420 seconds / 0.5s per step = 840 steps for full orbit
                true,
            );

            log::info!("Moon trajectory: {} points, orbit closes: {}, zoom_level: {}",
                moon_trajectory.len(), moon_orbit_closes, zoom_level);

            if !moon_trajectory.is_empty() {
                log::info!("First trajectory point: ({}, {})",
                    moon_trajectory[0].position.x, moon_trajectory[0].position.y);
                if let Some(last) = moon_trajectory.last() {
                    log::info!("Last trajectory point: ({}, {})",
                        last.position.x, last.position.y);
                }
                // Calculate what the line thickness will be
                let base_thickness = 8.0;
                let scaled_thickness = base_thickness * zoom_level.powf(0.8);
                log::info!("Moon trajectory line thickness: {} (base: {}, zoom: {})",
                    scaled_thickness, base_thickness, zoom_level);
            }

            // Draw moon trajectory in cyan/light blue
            let moon_color = if moon_orbit_closes {
                Color::new(0.0, 0.8, 1.0, 0.6) // Bright cyan if orbit closes
            } else {
                Color::new(0.5, 0.5, 1.0, 0.6) // Light purple if orbit doesn't close
            };

            self.trajectory_predictor.draw_trajectory(&moon_trajectory, moon_color, moon_orbit_closes, zoom_level, None);
            log::info!("Drew moon trajectory with {} points in color: {:?}", moon_trajectory.len(), moon_color);
        } else {
            log::warn!("Not enough planets for moon trajectory: {}", all_planets.len());
        }

        // MILESTONE-BASED TRAJECTORY SYSTEM (Much simpler and more efficient!)
        // Calculate 10 milestones, 300 steps (150s) apart
        // Wait until milestone 5 reached, then calculate next 10
        // Detect orbit when milestones loop back to start
        if let Some(rocket) = self.world.get_active_rocket() {
            let is_idle = self.idle_timer > GameConstants::TRAJECTORY_IDLE_EXPAND_SECONDS;

            // STEP 1: Calculate initial 10 milestones when first going idle
            if is_idle && self.milestone_nodes.is_empty() {
                log::info!("Calculating initial 10 milestone nodes (300 steps / 150s apart)");

                // Calculate trajectory in ONE shot: 10 milestones × 300 steps = 3000 total steps
                let (full_trajectory, _) = self.trajectory_predictor.predict_trajectory(
                    rocket,
                    &all_planets,
                    0.5,
                    10 * self.milestone_step_size, // 3000 steps total
                    false,
                );

                // Extract every 300th point as a milestone
                for i in 0..10 {
                    let idx = i * self.milestone_step_size;
                    if idx < full_trajectory.len() {
                        self.milestone_nodes.push(full_trajectory[idx].clone());
                    }
                }

                log::info!("Calculated {} milestone nodes", self.milestone_nodes.len());
            }

            // STEP 2: Check if rocket reached milestone #5, calculate next 10 milestones
            if is_idle && !self.milestone_nodes.is_empty() && !self.trajectory_orbit_detected {
                // Find distance to milestone #5
                if self.milestone_nodes.len() >= 5 {
                    let milestone_5_pos = self.milestone_nodes[4].position; // Index 4 = milestone #5
                    let distance_to_milestone_5 = (rocket.position() - milestone_5_pos).length();

                    // If close to milestone #5, calculate next 10 milestones
                    if distance_to_milestone_5 < 50.0 && self.current_milestone_index < 5 {
                        self.current_milestone_index = 5;
                        log::info!("Reached milestone #5, calculating next 10 milestones");

                        // Calculate next 10 milestones starting from last milestone
                        let last_milestone = self.milestone_nodes.last().unwrap();
                        let (next_trajectory, _) = self.trajectory_predictor.predict_trajectory_from_state(
                            last_milestone.position,
                            last_milestone.velocity,
                            rocket.current_mass(),
                            &all_planets,
                            0.5,
                            10 * self.milestone_step_size, // Next 3000 steps
                            false,
                        );

                        // Extract every 300th point as a milestone
                        let mut new_milestones = Vec::new();
                        for i in 1..=10 {  // Start at 1 to skip duplicate of last milestone
                            let idx = i * self.milestone_step_size;
                            if idx < next_trajectory.len() {
                                let mut milestone = next_trajectory[idx].clone();
                                milestone.time += last_milestone.time;
                                new_milestones.push(milestone);
                            }
                        }

                        // STEP 3: Check for orbit (do new milestones loop back to first milestones?)
                        if !new_milestones.is_empty() && self.milestone_nodes.len() >= 10 {
                            let mut loops_back = true;
                            let tolerance = 50.0; // Position within 50 units

                            // Compare new milestone positions with first 10 milestone positions
                            for i in 0..new_milestones.len().min(10) {
                                let new_pos = new_milestones[i].position;
                                let old_pos = self.milestone_nodes[i].position;
                                let distance = (new_pos - old_pos).length();

                                if distance > tolerance {
                                    loops_back = false;
                                    break;
                                }
                            }

                            if loops_back {
                                self.trajectory_orbit_detected = true;
                                log::info!("ORBIT DETECTED! Trajectory loops back to start. Total milestones: {}", self.milestone_nodes.len());
                            } else {
                                // Add new milestones to the list
                                self.milestone_nodes.extend(new_milestones);
                                log::info!("Added {} new milestones. Total: {}", new_milestones.len(), self.milestone_nodes.len());
                            }
                        } else {
                            // Add new milestones to the list
                            self.milestone_nodes.extend(new_milestones);
                            log::info!("Added {} new milestones. Total: {}", new_milestones.len(), self.milestone_nodes.len());
                        }
                    }
                }
            }

            // STEP 4: Draw milestones as simple circles
            if !self.milestone_nodes.is_empty() {
                // Collect milestone positions for drawing
                let milestone_positions: Vec<Vec2> = self.milestone_nodes
                    .iter()
                    .map(|node| node.position)
                    .collect();

                // Draw trajectory color based on orbit detection
                let trajectory_color = if self.trajectory_orbit_detected {
                    Color::new(0.0, 1.0, 0.0, 0.7) // Green - orbit detected!
                } else {
                    Color::new(1.0, 1.0, 0.0, 0.7) // Yellow - still exploring
                };

                // Draw milestones as fixed markers (pass empty trajectory, just show markers)
                self.trajectory_predictor.draw_trajectory(
                    &[],  // No trajectory line, just markers
                    trajectory_color,
                    self.trajectory_orbit_detected,
                    zoom_level,
                    Some(&milestone_positions),
                );
            } else if !is_idle {
                // Before idle threshold: show short predictive trajectory (100s)
                let (short_trajectory, intersects) = self.trajectory_predictor.predict_trajectory(
                    rocket,
                    &all_planets,
                    0.5,
                    200, // 200 steps = 100 seconds of short trajectory
                    false,
                );

                // Draw short trajectory with dynamic markers
                let trajectory_color = if intersects {
                    Color::new(0.0, 1.0, 0.0, 0.7) // Green
                } else {
                    Color::new(1.0, 1.0, 0.0, 0.7) // Yellow
                };

                self.trajectory_predictor.draw_trajectory(
                    &short_trajectory,
                    trajectory_color,
                    intersects,
                    zoom_level,
                    None, // Use dynamic markers for short trajectory
                );
            }
        }

        // Reset to default camera for HUD
                // Fully cached! Lock trajectory and track node-to-node travel
                if !self.trajectory_locked {
                    self.trajectory_locked = true;

                    // Capture fixed marker positions (every 20th point)
                    self.fixed_marker_positions.clear();
                    for (i, point) in self.cached_trajectory_nodes.iter().enumerate() {
                        if i % 20 == 0 {
                            self.fixed_marker_positions.push(point.position);
                        }
                    }

                    log::info!("Trajectory LOCKED - rocket will follow exact predicted path node-to-node ({} fixed markers)",
                        self.fixed_marker_positions.len());
                }

                // Track which node the rocket is traveling toward
                // Nodes are marked every 20 points, so we track progress toward marker nodes
                let marker_interval = 20;

                // Find distance to current target node (every 20th point is a visible marker)
                if self.current_node_index < self.cached_trajectory_nodes.len() {
                    let target_node_idx = (self.current_node_index / marker_interval) * marker_interval;
                    if target_node_idx < self.cached_trajectory_nodes.len() {
                        let target_pos = self.cached_trajectory_nodes[target_node_idx].position;
                        let distance_to_node = (rocket.position() - target_pos).length();

                        // If close enough to node, advance to next marker node
                        if distance_to_node < 30.0 {
                            let next_marker = target_node_idx + marker_interval;
                            if next_marker < self.cached_trajectory_nodes.len() {
                                self.current_node_index = next_marker;
                                log::info!("Reached node {}, advancing to next marker at index {}",
                                    target_node_idx, next_marker);
                            }
                        }
                    }
                }

                // Consume trajectory points as rocket passes them
                // Remove points that are behind the rocket
                let mut points_to_consume = 0;
                for (idx, point) in self.cached_trajectory_nodes.iter().enumerate().skip(self.consumed_trajectory_start) {
                    let distance = (rocket.position() - point.position).length();
                    if distance < 10.0 && idx > self.consumed_trajectory_start {
                        points_to_consume = idx;
                    } else if idx > points_to_consume + 5 {
                        // Stop checking once we're well ahead
                        break;
                    }
                }

                if points_to_consume > self.consumed_trajectory_start {
                    self.consumed_trajectory_start = points_to_consume;
                    log::debug!("Consumed trajectory up to index {}", points_to_consume);
                }

                // Check if trajectory self-intersects by examining cached nodes
                let mut intersects = false;
                if self.cached_trajectory_nodes.len() > 20 {
                    for i in 20..self.cached_trajectory_nodes.len() {
                        let pos = self.cached_trajectory_nodes[i].position;
                        // Check against early trajectory points
                        for j in 0..(i.saturating_sub(20)) {
                            let distance = (self.cached_trajectory_nodes[j].position - pos).length();
                            if distance < 50.0 {
                                intersects = true;
                                break;
                            }
                        }
                        if intersects { break; }
                    }
                }

                (self.cached_trajectory_nodes.clone(), intersects)
            } else {
                // OPTIMIZATION: Incrementally cache FIVE nodes per frame (was 1)
                // Reaches full cache 5x faster = less time in expensive preview phase
                let cached_count = self.cached_trajectory_nodes.len();
                let nodes_to_add = 5.min(trajectory_steps - cached_count); // Don't exceed target

                // Calculate next 5 nodes from the last cached position
                let last_cached = &self.cached_trajectory_nodes[cached_count - 1];
                let (next_nodes, _) = self.trajectory_predictor.predict_trajectory_from_state(
                    last_cached.position,
                    last_cached.velocity,
                    self.cached_rocket_mass,
                    &all_planets,
                    0.5,
                    nodes_to_add, // Add 5 nodes at once
                    false, // Don't check intersection during incremental add
                );

                // Add all new nodes to cache (with adjusted time)
                for mut new_node in next_nodes.into_iter().take(nodes_to_add) {
                    let last_time = self.cached_trajectory_nodes.last().unwrap().time;
                    new_node.time = last_time + 0.5; // Each node is 0.5s ahead
                    self.cached_trajectory_nodes.push(new_node);
                }

                let new_cached_count = self.cached_trajectory_nodes.len();
                if new_cached_count % 50 == 0 || new_cached_count == trajectory_steps {
                    log::info!("Trajectory caching progress: {}/{} nodes ({:.1}%)",
                        new_cached_count, trajectory_steps,
                        (new_cached_count as f32 / trajectory_steps as f32) * 100.0);
                }

                // ORBIT REPETITION DETECTION (OPTIMIZED)
                // Only check at specific milestones to reduce per-frame overhead
                // Milestones: 200, 400, 600, 800 nodes
                // This gives ~15% performance improvement during caching phase
                let is_milestone = matches!(new_cached_count, 200 | 400 | 600 | 800);

                if is_milestone && new_cached_count < trajectory_steps {
                    // Check if trajectory is repeating (completed one full orbit and starting to loop)
                    let check_window = 20; // Check last 20 points against first 20 points
                    if new_cached_count >= check_window * 2 {
                        let mut is_repeating = true;
                        let tolerance = 15.0; // Position must be within 15 units

                        // Compare last N points with first N points
                        for i in 0..check_window {
                            let start_pos = self.cached_trajectory_nodes[i].position;
                            let recent_idx = new_cached_count - check_window + i;
                            let recent_pos = self.cached_trajectory_nodes[recent_idx].position;
                            let distance = (start_pos - recent_pos).length();

                            if distance > tolerance {
                                is_repeating = false;
                                break;
                            }
                        }

                        if is_repeating {
                            // Orbit is repeating! Lock it now, truncate to one complete orbit
                            log::info!("STABLE ORBIT DETECTED at {} nodes (milestone check) - locking trajectory immediately!", new_cached_count);

                            // Trim trajectory to just the completed orbit (exclude the repeating part)
                            self.cached_trajectory_nodes.truncate(new_cached_count - check_window);

                            // Force lock by setting count to trajectory_steps
                            // This will trigger the locking logic in the next frame
                            while self.cached_trajectory_nodes.len() < trajectory_steps {
                                // Pad with last node to reach trajectory_steps
                                let last = self.cached_trajectory_nodes.last().unwrap().clone();
                                self.cached_trajectory_nodes.push(last);
                            }
                        }
                    }
                }

                // OPTIMIZATION: Preview trajectory caching (MASSIVE performance boost)
                // Calculate preview ONCE and lock it - don't recalculate until cache grows
                let remaining_steps = trajectory_steps - new_cached_count;

                if remaining_steps > 0 {
                    // Only recalculate preview if:
                    // 1. Preview is empty (first time), OR
                    // 2. Preview is longer than remaining steps (cache has grown, preview is stale)
                    let need_new_preview = self.cached_preview_trajectory.is_empty() ||
                                          self.cached_preview_trajectory.len() > remaining_steps;

                    if need_new_preview {
                        // Calculate preview trajectory ONCE (expensive operation)
                        let last_cached_now = self.cached_trajectory_nodes.last().unwrap();
                        let (mut remaining_trajectory, intersects) = self.trajectory_predictor.predict_trajectory_from_state(
                            last_cached_now.position,
                            last_cached_now.velocity,
                            self.cached_rocket_mass,
                            &all_planets,
                            0.5,
                            remaining_steps,
                            true,
                        );

                        // Adjust time values in remaining trajectory
                        for point in &mut remaining_trajectory {
                            point.time += last_cached_now.time;
                        }

                        // Cache the preview and LOCK IT
                        self.cached_preview_trajectory = remaining_trajectory;
                        self.preview_frames_since_recalc = 0;

                        log::info!("Calculated NEW preview trajectory ({} points) - will remain stable until cache grows",
                            self.cached_preview_trajectory.len());

                        // Concatenate cached + preview for display
                        let mut full_trajectory = self.cached_trajectory_nodes.clone();
                        full_trajectory.append(&mut self.cached_preview_trajectory.clone());

                        (full_trajectory, intersects)
                    } else {
                        // Reuse LOCKED preview (no recalculation!)
                        self.preview_frames_since_recalc += 1;

                        // Concatenate cached + preview for display
                        let mut full_trajectory = self.cached_trajectory_nodes.clone();
                        full_trajectory.append(&mut self.cached_preview_trajectory.clone());

                        // Check intersection on preview (lightweight)
                        let intersects = false; // Will be accurate once fully cached
                        (full_trajectory, intersects)
                    }
                } else {
                    // All cached, check for intersection
                    let intersects = false; // Will be checked in next frame when fully cached
                    (self.cached_trajectory_nodes.clone(), intersects)
                }
            };

            // Only draw trajectory from consumed_trajectory_start onward (skip already-traveled segments)
            let trajectory_to_draw = if self.trajectory_locked && self.consumed_trajectory_start > 0 {
                // Skip consumed trajectory points
                let start_idx = self.consumed_trajectory_start.min(trajectory_points.len());
                &trajectory_points[start_idx..]
            } else {
                // Draw full trajectory if not locked or nothing consumed yet
                &trajectory_points[..]
            };

            // Draw trajectory with color based on whether it self-intersects (completes orbit)
            let trajectory_color = if self_intersects {
                Color::new(0.0, 1.0, 0.0, 0.7) // Green if orbit closes
            } else {
                Color::new(1.0, 1.0, 0.0, 0.7) // Yellow if orbit is open
            };

            // Pass fixed markers if trajectory is locked, otherwise None
            let fixed_markers = if self.trajectory_locked && !self.fixed_marker_positions.is_empty() {
                Some(self.fixed_marker_positions.as_slice())
            } else {
                None
            };

            self.trajectory_predictor.draw_trajectory(trajectory_to_draw, trajectory_color, self_intersects, zoom_level, fixed_markers);
        }

        // Reset to default camera for HUD
        set_default_camera();

        // Render HUD
        if let Some(rocket) = self.world.get_active_rocket() {
            self.hud.draw_rocket_stats(rocket, self.selected_thrust_level);
        } else {
            self.hud.draw_message("No active rocket");
        }

        // Draw pause indicator if paused (but not if showing controls)
        if self.is_paused && !self.show_controls {
            let screen_width = screen_width();
            let screen_height = screen_height();
            let pause_text = "PAUSED";
            let font_size = 48.0;
            let text_dims = measure_text(pause_text, None, font_size as u16, 1.0);
            draw_text(
                pause_text,
                screen_width / 2.0 - text_dims.width / 2.0,
                screen_height / 2.0,
                font_size,
                WHITE,
            );
        }

        // Draw controls button in top-right corner
        let screen_w = screen_width();
        let button_x = screen_w - 50.0;
        let button_y = 10.0;
        let button_w = 40.0;
        let button_h = 30.0;

        // Button background
        draw_rectangle(button_x, button_y, button_w, button_h, Color::new(0.2, 0.2, 0.2, 0.8));
        // Button border
        draw_rectangle_lines(button_x, button_y, button_w, button_h, 2.0, WHITE);
        // Button text "..."
        let button_text = "...";
        let button_text_dims = measure_text(button_text, None, 20, 1.0);
        draw_text(
            button_text,
            button_x + button_w / 2.0 - button_text_dims.width / 2.0,
            button_y + button_h / 2.0 + button_text_dims.height / 2.0,
            20.0,
            WHITE,
        );

        // Draw controls popup if showing
        if self.show_controls {
            let screen_h = screen_height();
            let popup_w = 400.0;
            let popup_h = 500.0;
            let popup_x = screen_w / 2.0 - popup_w / 2.0;
            let popup_y = screen_h / 2.0 - popup_h / 2.0;

            // Semi-transparent overlay
            draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.5));

            // Popup background
            draw_rectangle(popup_x, popup_y, popup_w, popup_h, Color::new(0.1, 0.1, 0.1, 0.95));
            // Popup border
            draw_rectangle_lines(popup_x, popup_y, popup_w, popup_h, 3.0, WHITE);

            // Title
            let title = "CONTROLS";
            let title_size = 32.0;
            let title_dims = measure_text(title, None, title_size as u16, 1.0);
            draw_text(
                title,
                popup_x + popup_w / 2.0 - title_dims.width / 2.0,
                popup_y + 40.0,
                title_size,
                WHITE,
            );

            // Controls list
            let controls = [
                ("COMMA", "Decrease thrust -5%"),
                ("PERIOD", "Increase thrust +5%"),
                ("SPACE", "Apply thrust"),
                ("A / LEFT", "Rotate right"),
                ("D / RIGHT", "Rotate left"),
                ("Q", "Zoom in"),
                ("E", "Zoom out"),
                ("MOUSE WHEEL", "Zoom"),
                ("L", "Convert to satellite"),
                ("T", "Launch new rocket"),
                ("P", "Pause/Unpause"),
                ("F5", "Quick save"),
                ("ENTER", "Toggle this menu"),
                ("ESC", "Return to menu / Close this"),
            ];

            let mut y = popup_y + 80.0;
            let font_size = 18.0;
            let line_height = 35.0;

            for (key, action) in &controls {
                // Draw key
                draw_text(
                    key,
                    popup_x + 30.0,
                    y,
                    font_size,
                    Color::new(0.8, 0.8, 1.0, 1.0), // Light blue
                );

                // Draw action
                draw_text(
                    action,
                    popup_x + 200.0,
                    y,
                    font_size,
                    WHITE,
                );

                y += line_height;
            }

            // Footer text
            let footer = "Click outside or press ESC to close";
            let footer_dims = measure_text(footer, None, 14, 1.0);
            draw_text(
                footer,
                popup_x + popup_w / 2.0 - footer_dims.width / 2.0,
                popup_y + popup_h - 20.0,
                14.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn game_time(&self) -> f32 {
        self.game_time
    }

    /// Reset trajectory milestones (called on user input or zoom)
    fn reset_trajectory_cache(&mut self) {
        self.milestone_nodes.clear();
        self.current_milestone_index = 0;
        self.trajectory_orbit_detected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests requiring SFML resources (Font) are limited
    // Would need mock or test fixtures

    #[test]
    fn test_game_time_tracking() {
        // This test would require proper setup with Font
        // Simplified test structure shown
    }
}
