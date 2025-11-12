// Multiplayer Saves Menu - New game and load game selection for multiplayer
// Based on SavesMenu but reads from saves/multi/ folder

use macroquad::prelude::*;

use crate::ui::Button;

/// Result from multiplayer saves menu interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiplayerSavesMenuResult {
    None,
    NewGame(u16), // New game with port
    LoadGame(String, u16), // Load game with save name and port
    Back,
}

/// Multiplayer saves menu for creating new games or loading existing ones
pub struct MultiplayerSavesMenu {
    title_text: String,
    title_position: Vec2,
    title_font_size: f32,
    new_game_button: Button,
    back_button: Button,
    save_buttons: Vec<Button>,
    save_names: Vec<String>,
    window_size: Vec2,
    port_input: String,
    error_message: Option<String>,
}

impl MultiplayerSavesMenu {
    pub fn new(window_size: Vec2) -> Self {
        // Title
        let title_text = "Select Multiplayer Save".to_string();
        let title_font_size = 48.0;

        // Calculate title position (centered)
        let text_dims = measure_text(&title_text, None, title_font_size as u16, 1.0);
        let title_position = Vec2::new(
            window_size.x / 2.0 - text_dims.width / 2.0,
            80.0 + text_dims.height,
        );

        // Button dimensions
        let button_width = 350.0;
        let button_height = 50.0;
        let start_y = 240.0; // Leave space for port input

        // New Game button
        let new_game_button = Button::new(
            Vec2::new(
                window_size.x / 2.0 - button_width / 2.0,
                start_y,
            ),
            Vec2::new(button_width, button_height),
            "New Multiplayer Game",
            Color::from_rgba(50, 150, 50, 255),
        );

        // Back button
        let back_button = Button::new(
            Vec2::new(50.0, window_size.y - 80.0),
            Vec2::new(150.0, 50.0),
            "Back",
            Color::from_rgba(100, 100, 100, 255),
        );

        MultiplayerSavesMenu {
            title_text,
            title_position,
            title_font_size,
            new_game_button,
            back_button,
            save_buttons: Vec::new(),
            save_names: Vec::new(),
            window_size,
            port_input: "7777".to_string(), // Default port
            error_message: None,
        }
    }

    /// Load available save files from saves/multi/ and create buttons
    pub fn refresh_saves(&mut self) {
        // Clear existing save buttons
        self.save_buttons.clear();
        self.save_names.clear();

        // Get save files from disk
        if let Ok(saves) = self.load_save_list() {
            let button_width = 350.0;
            let button_height = 50.0;
            let button_spacing = 60.0;
            let start_y = 320.0; // Below "New Game" button

            for (i, save_name) in saves.iter().enumerate() {
                let button = Button::new(
                    Vec2::new(
                        self.window_size.x / 2.0 - button_width / 2.0,
                        start_y + (i as f32 * button_spacing),
                    ),
                    Vec2::new(button_width, button_height),
                    save_name,
                    Color::from_rgba(70, 90, 120, 255),
                );

                self.save_buttons.push(button);
                self.save_names.push(save_name.clone());
            }
        }
    }

    /// Load list of save files from saves/multi/ directory
    fn load_save_list(&self) -> Result<Vec<String>, std::io::Error> {
        use std::fs;

        let saves_dir = "saves/multi";

        // Create saves/multi directory if it doesn't exist
        fs::create_dir_all(saves_dir)?;

        let mut saves = Vec::new();

        // Read all .sav files in saves/multi directory
        for entry in fs::read_dir(saves_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("sav") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    saves.push(file_name.to_string());
                }
            }
        }

        saves.sort();
        Ok(saves)
    }

    /// Update menu and handle input
    pub fn update(&mut self) -> MultiplayerSavesMenuResult {
        // Handle text input for port
        if let Some(key) = get_last_key_pressed() {
            match key {
                KeyCode::Key0 | KeyCode::Kp0 => self.port_input.push('0'),
                KeyCode::Key1 | KeyCode::Kp1 => self.port_input.push('1'),
                KeyCode::Key2 | KeyCode::Kp2 => self.port_input.push('2'),
                KeyCode::Key3 | KeyCode::Kp3 => self.port_input.push('3'),
                KeyCode::Key4 | KeyCode::Kp4 => self.port_input.push('4'),
                KeyCode::Key5 | KeyCode::Kp5 => self.port_input.push('5'),
                KeyCode::Key6 | KeyCode::Kp6 => self.port_input.push('6'),
                KeyCode::Key7 | KeyCode::Kp7 => self.port_input.push('7'),
                KeyCode::Key8 | KeyCode::Kp8 => self.port_input.push('8'),
                KeyCode::Key9 | KeyCode::Kp9 => self.port_input.push('9'),
                KeyCode::Backspace => {
                    self.port_input.pop();
                }
                _ => {}
            }

            // Limit port length
            if self.port_input.len() > 5 {
                self.port_input.truncate(5);
            }
        }

        let mouse_pressed = is_mouse_button_down(MouseButton::Left);

        // Check new game button
        if self.new_game_button.update(mouse_pressed) {
            // Validate port
            match self.port_input.parse::<u16>() {
                Ok(port) if port > 0 => {
                    self.error_message = None;
                    return MultiplayerSavesMenuResult::NewGame(port);
                }
                _ => {
                    self.error_message = Some("Invalid port number".to_string());
                }
            }
        }

        // Check back button
        if self.back_button.update(mouse_pressed) {
            return MultiplayerSavesMenuResult::Back;
        }

        // Check save file buttons
        for (i, button) in self.save_buttons.iter_mut().enumerate() {
            if button.update(mouse_pressed) {
                if let Some(save_name) = self.save_names.get(i) {
                    // Validate port
                    match self.port_input.parse::<u16>() {
                        Ok(port) if port > 0 => {
                            self.error_message = None;
                            return MultiplayerSavesMenuResult::LoadGame(save_name.clone(), port);
                        }
                        _ => {
                            self.error_message = Some("Invalid port number".to_string());
                        }
                    }
                }
            }
        }

        MultiplayerSavesMenuResult::None
    }

    /// Draw the menu
    pub fn draw(&self) {
        // Draw title
        draw_text(
            &self.title_text,
            self.title_position.x,
            self.title_position.y,
            self.title_font_size,
            WHITE,
        );

        // Port input box
        let input_y = 160.0;
        let input_width = 200.0;
        let input_height = 40.0;
        let input_x = self.window_size.x / 2.0 - input_width / 2.0;

        // Draw input box background
        draw_rectangle(input_x, input_y, input_width, input_height, Color::new(0.2, 0.2, 0.3, 1.0));
        draw_rectangle_lines(input_x, input_y, input_width, input_height, 2.0, WHITE);

        // Draw label
        let label = "Port:";
        let label_size = 20.0;
        let label_dims = measure_text(label, None, label_size as u16, 1.0);
        draw_text(label, input_x - label_dims.width - 15.0, input_y + 27.0, label_size, WHITE);

        // Draw port text
        let port_text_size = 25.0;
        let port_dims = measure_text(&self.port_input, None, port_text_size as u16, 1.0);
        let port_x = input_x + input_width / 2.0 - port_dims.width / 2.0;
        draw_text(&self.port_input, port_x, input_y + 28.0, port_text_size, WHITE);

        // Draw cursor blink
        if (get_time() * 2.0) as i32 % 2 == 0 {
            let cursor_x = port_x + port_dims.width + 5.0;
            draw_rectangle(cursor_x, input_y + 8.0, 2.0, 25.0, WHITE);
        }

        // Draw buttons
        self.new_game_button.draw();

        for button in &self.save_buttons {
            button.draw();
        }

        self.back_button.draw();

        // Draw error message if any
        if let Some(ref error) = self.error_message {
            let error_size = 24.0;
            let error_dims = measure_text(error, None, error_size as u16, 1.0);
            let error_x = self.window_size.x / 2.0 - error_dims.width / 2.0;
            let error_y = self.window_size.y - 150.0;
            draw_text(error, error_x, error_y, error_size, RED);
        }

        // Info text
        let info = "Enter port and select a save or create new game";
        let info_size = 18.0;
        let info_dims = measure_text(info, None, info_size as u16, 1.0);
        let info_x = self.window_size.x / 2.0 - info_dims.width / 2.0;
        let info_y = self.window_size.y - 50.0;
        draw_text(info, info_x, info_y, info_size, LIGHTGRAY);
    }
}
