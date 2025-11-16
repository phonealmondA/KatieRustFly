// Map Selection Menu - Choose which map to play
// Allows player to select between different planetary configurations

use macroquad::prelude::*;

use crate::map_config::MapConfiguration;
use crate::ui::Button;

/// Result from map selection menu interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapSelectionResult {
    None,
    MapSelected(String), // Returns the map name
    Back,
}

/// Map selection menu for choosing planetary configurations
pub struct MapSelectionMenu {
    title_text: String,
    title_position: Vec2,
    title_font_size: f32,
    back_button: Button,
    map_buttons: Vec<Button>,
    map_names: Vec<String>,
    map_descriptions: Vec<String>,
    window_size: Vec2,
}

impl MapSelectionMenu {
    pub fn new(window_size: Vec2) -> Self {
        // Title
        let title_text = "Select Map".to_string();
        let title_font_size = 48.0;

        // Calculate title position (centered)
        let text_dims = measure_text(&title_text, None, title_font_size as u16, 1.0);
        let title_position = Vec2::new(
            window_size.x / 2.0 - text_dims.width / 2.0,
            80.0 + text_dims.height,
        );

        // Back button
        let back_button = Button::new(
            Vec2::new(50.0, window_size.y - 80.0),
            Vec2::new(150.0, 50.0),
            "Back",
            Color::from_rgba(100, 100, 100, 255),
        );

        let mut menu = MapSelectionMenu {
            title_text,
            title_position,
            title_font_size,
            back_button,
            map_buttons: Vec::new(),
            map_names: Vec::new(),
            map_descriptions: Vec::new(),
            window_size,
        };

        // Initialize map buttons
        menu.load_maps();

        menu
    }

    /// Load available maps and create buttons
    fn load_maps(&mut self) {
        // Clear existing buttons
        self.map_buttons.clear();
        self.map_names.clear();
        self.map_descriptions.clear();

        // Get all available maps
        let maps = MapConfiguration::all_maps();

        let button_width = 500.0;
        let button_height = 80.0;
        let button_spacing = 120.0;
        let start_y = 180.0;

        for (i, map) in maps.iter().enumerate() {
            let button = Button::new(
                Vec2::new(
                    self.window_size.x / 2.0 - button_width / 2.0,
                    start_y + (i as f32 * button_spacing),
                ),
                Vec2::new(button_width, button_height),
                &map.name,
                Color::from_rgba(50, 100, 150, 255),
            );

            self.map_buttons.push(button);
            self.map_names.push(map.name.clone());
            self.map_descriptions.push(map.description.clone());
        }
    }

    /// Update menu and handle input
    pub fn update(&mut self) -> MapSelectionResult {
        let mouse_pressed = is_mouse_button_down(MouseButton::Left);

        // Check back button
        if self.back_button.update(mouse_pressed) {
            return MapSelectionResult::Back;
        }

        // Check map buttons
        for (i, button) in self.map_buttons.iter_mut().enumerate() {
            if button.update(mouse_pressed) {
                return MapSelectionResult::MapSelected(self.map_names[i].clone());
            }
        }

        MapSelectionResult::None
    }

    /// Render the menu
    pub fn render(&self) {
        clear_background(BLACK);

        // Draw title
        draw_text(
            &self.title_text,
            self.title_position.x,
            self.title_position.y,
            self.title_font_size,
            WHITE,
        );

        // Draw map buttons
        for (i, button) in self.map_buttons.iter().enumerate() {
            button.draw();

            // Draw description below button
            let desc_x = button.position().x + 10.0;
            let desc_y = button.position().y + button.size().y + 20.0;
            draw_text(
                &self.map_descriptions[i],
                desc_x,
                desc_y,
                20.0,
                Color::from_rgba(180, 180, 180, 255),
            );
        }

        // Draw back button
        self.back_button.draw();

        // Draw instructions
        let instructions = "Click a map to start a new game";
        let inst_size = 24.0;
        let inst_dims = measure_text(instructions, None, inst_size as u16, 1.0);
        draw_text(
            instructions,
            self.window_size.x / 2.0 - inst_dims.width / 2.0,
            self.window_size.y - 150.0,
            inst_size,
            Color::from_rgba(200, 200, 200, 255),
        );
    }
}
