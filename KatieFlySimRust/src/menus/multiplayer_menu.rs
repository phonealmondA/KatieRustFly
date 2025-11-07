// Multiplayer Menu - Choose between local and online multiplayer

use macroquad::prelude::*;
use crate::ui::Button;
use crate::game_state::GameState;

/// Multiplayer menu
pub struct MultiplayerMenu {
    title: String,
    local_button: Button,
    online_button: Button,
    split_screen_button: Button,
    back_button: Button,
}

impl MultiplayerMenu {
    pub fn new() -> Self {
        let screen_width = screen_width();
        let screen_height = screen_height();
        let button_width = 300.0;
        let button_height = 50.0;
        let center_x = screen_width / 2.0 - button_width / 2.0;
        let start_y = screen_height / 2.0 - 100.0;
        let spacing = 70.0;

        MultiplayerMenu {
            title: "Multiplayer".to_string(),
            local_button: Button::new(
                Vec2::new(center_x, start_y),
                Vec2::new(button_width, button_height),
                "Local Multiplayer",
                Color::from_rgba(50, 100, 150, 255),
            ),
            online_button: Button::new(
                Vec2::new(center_x, start_y + spacing),
                Vec2::new(button_width, button_height),
                "Online Multiplayer",
                Color::from_rgba(50, 120, 100, 255),
            ),
            split_screen_button: Button::new(
                Vec2::new(center_x, start_y + spacing * 2.0),
                Vec2::new(button_width, button_height),
                "Split Screen",
                Color::from_rgba(100, 50, 150, 255),
            ),
            back_button: Button::new(
                Vec2::new(center_x, start_y + spacing * 3.0),
                Vec2::new(button_width, button_height),
                "Back",
                Color::from_rgba(120, 50, 50, 255),
            ),
        }
    }

    /// Update menu and return new game state if button clicked
    pub fn update(&mut self) -> Option<GameState> {
        let mouse_pressed = is_mouse_button_down(MouseButton::Left);

        // Check for button clicks
        if self.local_button.update(mouse_pressed) {
            // Local multiplayer not yet implemented
            return Some(GameState::MainMenu);
        }

        if self.online_button.update(mouse_pressed) {
            return Some(GameState::OnlineMultiplayerMenu);
        }

        if self.split_screen_button.update(mouse_pressed) {
            // Split screen mode
            return Some(GameState::SplitScreen);
        }

        if self.back_button.update(mouse_pressed) {
            return Some(GameState::MainMenu);
        }

        None
    }

    /// Render menu
    pub fn render(&self) {
        // Clear background
        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        // Draw title
        let title_size = 48.0;
        let title_dims = measure_text(&self.title, None, title_size as u16, 1.0);
        let title_x = screen_width() / 2.0 - title_dims.width / 2.0;
        let title_y = screen_height() / 2.0 - 200.0;

        draw_text(&self.title, title_x, title_y, title_size, WHITE);

        // Draw buttons
        self.local_button.draw();
        self.online_button.draw();
        self.split_screen_button.draw();
        self.back_button.draw();

        // Draw note about unimplemented features
        let note = "Note: Multiplayer features are in development";
        let note_size = 20.0;
        let note_dims = measure_text(note, None, note_size as u16, 1.0);
        let note_x = screen_width() / 2.0 - note_dims.width / 2.0;
        let note_y = screen_height() - 50.0;
        draw_text(note, note_x, note_y, note_size, Color::new(0.7, 0.7, 0.0, 1.0));
    }
}

impl Default for MultiplayerMenu {
    fn default() -> Self {
        Self::new()
    }
}
