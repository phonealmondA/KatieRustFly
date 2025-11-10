// Multiplayer Menu - Choose between split-screen and online multiplayer
// Displays options for local split-screen or online multiplayer modes

use macroquad::prelude::*;
use crate::ui::Button;

/// Result from multiplayer menu interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiplayerMenuResult {
    None,
    SplitScreen,
    OnlineMultiplayer,
    Back,
}

/// Multiplayer menu for selecting game mode
pub struct MultiplayerMenu {
    title_text: String,
    title_position: Vec2,
    title_font_size: f32,
    split_screen_button: Button,
    online_button: Button,
    back_button: Button,
    window_size: Vec2,
}

impl MultiplayerMenu {
    pub fn new(window_size: Vec2) -> Self {
        // Title
        let title_text = "Multiplayer".to_string();
        let title_font_size = 48.0;

        // Calculate title position (centered)
        let text_dims = measure_text(&title_text, None, title_font_size as u16, 1.0);
        let title_position = Vec2::new(
            window_size.x / 2.0 - text_dims.width / 2.0,
            80.0 + text_dims.height,
        );

        // Button dimensions
        let button_width = 400.0;
        let button_height = 60.0;
        let button_spacing = 80.0;
        let start_y = 200.0;

        // Split-screen button
        let split_screen_button = Button::new(
            Vec2::new(
                window_size.x / 2.0 - button_width / 2.0,
                start_y,
            ),
            Vec2::new(button_width, button_height),
            "Split-Screen (Local)",
            Color::from_rgba(50, 150, 50, 255),
        );

        // Online multiplayer button
        let online_button = Button::new(
            Vec2::new(
                window_size.x / 2.0 - button_width / 2.0,
                start_y + button_spacing,
            ),
            Vec2::new(button_width, button_height),
            "Online Multiplayer",
            Color::from_rgba(50, 100, 200, 255),
        );

        // Back button
        let back_button = Button::new(
            Vec2::new(50.0, window_size.y - 80.0),
            Vec2::new(150.0, 50.0),
            "Back",
            Color::from_rgba(100, 100, 100, 255),
        );

        MultiplayerMenu {
            title_text,
            title_position,
            title_font_size,
            split_screen_button,
            online_button,
            back_button,
            window_size,
        }
    }

    /// Update menu and handle input
    pub fn update(&mut self) -> MultiplayerMenuResult {
        let mouse_pressed = is_mouse_button_down(MouseButton::Left);

        // Check split-screen button
        if self.split_screen_button.update(mouse_pressed) {
            return MultiplayerMenuResult::SplitScreen;
        }

        // Check online button
        if self.online_button.update(mouse_pressed) {
            return MultiplayerMenuResult::OnlineMultiplayer;
        }

        // Check back button
        if self.back_button.update(mouse_pressed) {
            return MultiplayerMenuResult::Back;
        }

        MultiplayerMenuResult::None
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

        // Draw description text
        let desc = "Choose your multiplayer mode";
        let desc_dims = measure_text(desc, None, 24, 1.0);
        draw_text(
            desc,
            self.window_size.x / 2.0 - desc_dims.width / 2.0,
            140.0,
            24.0,
            GRAY,
        );

        // Draw buttons
        self.split_screen_button.draw();
        self.online_button.draw();
        self.back_button.draw();

        // Draw mode descriptions
        let split_desc = "Play with a friend on the same device";
        let online_desc = "Host or join a game over the network";

        let split_desc_dims = measure_text(split_desc, None, 16, 1.0);
        let online_desc_dims = measure_text(online_desc, None, 16, 1.0);

        draw_text(
            split_desc,
            self.window_size.x / 2.0 - split_desc_dims.width / 2.0,
            270.0,
            16.0,
            LIGHTGRAY,
        );

        draw_text(
            online_desc,
            self.window_size.x / 2.0 - online_desc_dims.width / 2.0,
            350.0,
            16.0,
            LIGHTGRAY,
        );
    }
}
