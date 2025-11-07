// Online Multiplayer Menu - Host or join online games

use macroquad::prelude::*;
use crate::ui::Button;
use crate::game_state::GameState;

/// Online multiplayer menu
pub struct OnlineMultiplayerMenu {
    title: String,
    host_button: Button,
    join_button: Button,
    back_button: Button,
    ip_input: String,
}

impl OnlineMultiplayerMenu {
    pub fn new() -> Self {
        let scr_width = screen_width();
        let scr_height = screen_height();
        let button_width = 300.0;
        let button_height = 50.0;
        let center_x = scr_width / 2.0 - button_width / 2.0;
        let start_y = scr_height / 2.0 - 50.0;
        let spacing = 70.0;

        OnlineMultiplayerMenu {
            title: "Online Multiplayer".to_string(),
            host_button: Button::new(
                Vec2::new(center_x, start_y),
                Vec2::new(button_width, button_height),
                "Host Game",
                Color::from_rgba(50, 100, 150, 255),
            ),
            join_button: Button::new(
                Vec2::new(center_x, start_y + spacing * 2.0),
                Vec2::new(button_width, button_height),
                "Join Game",
                Color::from_rgba(50, 120, 100, 255),
            ),
            back_button: Button::new(
                Vec2::new(center_x, start_y + spacing * 3.0),
                Vec2::new(button_width, button_height),
                "Back",
                Color::from_rgba(120, 50, 50, 255),
            ),
            ip_input: "127.0.0.1".to_string(),
        }
    }

    /// Update menu and return new game state if button clicked
    pub fn update(&mut self) -> Option<GameState> {
        let mouse_pressed = is_mouse_button_down(MouseButton::Left);

        // Check for button clicks
        if self.host_button.update(mouse_pressed) {
            return Some(GameState::MultiplayerHost);
        }

        if self.join_button.update(mouse_pressed) {
            return Some(GameState::MultiplayerClient);
        }

        if self.back_button.update(mouse_pressed) {
            return Some(GameState::MultiplayerMenu);
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
        self.host_button.draw();
        self.join_button.draw();
        self.back_button.draw();

        // Draw IP input label
        let scr_width = screen_width();
        let scr_height = screen_height();
        let button_width = 300.0;
        let center_x = scr_width / 2.0 - button_width / 2.0;
        let start_y = scr_height / 2.0 - 50.0;
        let spacing = 70.0;

        let input_y = start_y + spacing;

        // Draw label
        let label = format!("Server IP: {}", self.ip_input);
        let label_size = 20.0;
        draw_text(&label, center_x, input_y + 25.0, label_size, WHITE);

        // Draw note about networking
        let note = "Note: Online multiplayer requires networking implementation";
        let note_size = 18.0;
        let note_dims = measure_text(note, None, note_size as u16, 1.0);
        let note_x = scr_width / 2.0 - note_dims.width / 2.0;
        let note_y = scr_height - 80.0;
        draw_text(note, note_x, note_y, note_size, Color::new(0.7, 0.7, 0.0, 1.0));

        let note2 = "Networking features are currently in development";
        let note2_dims = measure_text(note2, None, note_size as u16, 1.0);
        let note2_x = scr_width / 2.0 - note2_dims.width / 2.0;
        draw_text(note2, note2_x, note_y + 25.0, note_size, Color::new(0.7, 0.7, 0.0, 1.0));
    }

    /// Get entered IP address
    pub fn get_ip_address(&self) -> &str {
        &self.ip_input
    }

    /// Set IP address
    pub fn set_ip_address(&mut self, ip: String) {
        self.ip_input = ip;
    }
}

impl Default for OnlineMultiplayerMenu {
    fn default() -> Self {
        Self::new()
    }
}
