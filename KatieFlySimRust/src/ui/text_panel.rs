// TextPanel - Multi-line text display panel with background

use macroquad::prelude::*;

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Text panel configuration
#[derive(Debug, Clone)]
pub struct TextPanelConfig {
    pub position: Vec2,
    pub width: f32,
    pub height: f32,
    pub background_color: Color,
    pub border_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    pub padding: f32,
    pub line_spacing: f32,
    pub alignment: TextAlignment,
    pub show_border: bool,
    pub border_width: f32,
}

impl Default for TextPanelConfig {
    fn default() -> Self {
        TextPanelConfig {
            position: Vec2::new(10.0, 10.0),
            width: 300.0,
            height: 200.0,
            background_color: Color::new(0.0, 0.0, 0.0, 0.7),
            border_color: Color::new(1.0, 1.0, 1.0, 0.8),
            text_color: WHITE,
            font_size: 16.0,
            padding: 10.0,
            line_spacing: 5.0,
            alignment: TextAlignment::Left,
            show_border: true,
            border_width: 2.0,
        }
    }
}

/// TextPanel - Display multi-line text with background
pub struct TextPanel {
    config: TextPanelConfig,
    lines: Vec<String>,
    title: Option<String>,
    visible: bool,
}

impl TextPanel {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        let config = TextPanelConfig {
            position,
            width: size.x,
            height: size.y,
            ..Default::default()
        };

        TextPanel {
            config,
            lines: Vec::new(),
            title: None,
            visible: true,
        }
    }

    pub fn from_config(config: TextPanelConfig) -> Self {
        TextPanel {
            config,
            lines: Vec::new(),
            title: None,
            visible: true,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_background_color(mut self, color: Color) -> Self {
        self.config.background_color = color;
        self
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.config.border_color = color;
        self
    }

    pub fn with_text_color(mut self, color: Color) -> Self {
        self.config.text_color = color;
        self
    }

    // === Configuration ===

    pub fn set_position(&mut self, position: Vec2) {
        self.config.position = position;
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.config.width = width;
        self.config.height = height;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.config.background_color = color;
    }

    pub fn set_text_color(&mut self, color: Color) {
        self.config.text_color = color;
    }

    pub fn set_alignment(&mut self, alignment: TextAlignment) {
        self.config.alignment = alignment;
    }

    // === Content Management ===

    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|s| s.to_string()).collect();
    }

    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    pub fn add_line(&mut self, line: String) {
        self.lines.push(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    // === Rendering ===

    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        let pos = self.config.position;
        let width = self.config.width;
        let height = self.config.height;

        // Draw background
        draw_rectangle(
            pos.x,
            pos.y,
            width,
            height,
            self.config.background_color,
        );

        // Draw border
        if self.config.show_border {
            draw_rectangle_lines(
                pos.x,
                pos.y,
                width,
                height,
                self.config.border_width,
                self.config.border_color,
            );
        }

        // Calculate text area
        let text_x = pos.x + self.config.padding;
        let mut text_y = pos.y + self.config.padding;
        let text_width = width - self.config.padding * 2.0;

        // Draw title if present
        if let Some(ref title) = self.title {
            let title_size = self.config.font_size + 4.0;
            self.draw_line_with_alignment(
                title,
                text_x,
                text_y,
                text_width,
                title_size,
                Color::new(1.0, 1.0, 0.0, 1.0), // Yellow for title
            );
            text_y += title_size + self.config.line_spacing * 2.0;

            // Draw separator line
            draw_line(
                pos.x + self.config.padding,
                text_y - self.config.line_spacing,
                pos.x + width - self.config.padding,
                text_y - self.config.line_spacing,
                1.0,
                self.config.border_color,
            );
        }

        // Draw text lines
        for line in &self.lines {
            // Check if line fits in panel
            if text_y + self.config.font_size > pos.y + height - self.config.padding {
                // Draw "..." to indicate more content
                self.draw_line_with_alignment(
                    "...",
                    text_x,
                    text_y,
                    text_width,
                    self.config.font_size,
                    self.config.text_color,
                );
                break;
            }

            self.draw_line_with_alignment(
                line,
                text_x,
                text_y,
                text_width,
                self.config.font_size,
                self.config.text_color,
            );

            text_y += self.config.font_size + self.config.line_spacing;
        }
    }

    /// Draw a single line with alignment
    fn draw_line_with_alignment(
        &self,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
        font_size: f32,
        color: Color,
    ) {
        let text_dims = measure_text(text, None, font_size as u16, 1.0);

        let draw_x = match self.config.alignment {
            TextAlignment::Left => x,
            TextAlignment::Center => x + (width - text_dims.width) / 2.0,
            TextAlignment::Right => x + width - text_dims.width,
        };

        draw_text(text, draw_x, y + font_size, font_size, color);
    }

    /// Draw text that wraps to fit width
    pub fn draw_wrapped(&self) {
        if !self.visible {
            return;
        }

        let pos = self.config.position;
        let width = self.config.width;
        let height = self.config.height;

        // Draw background
        draw_rectangle(
            pos.x,
            pos.y,
            width,
            height,
            self.config.background_color,
        );

        // Draw border
        if self.config.show_border {
            draw_rectangle_lines(
                pos.x,
                pos.y,
                width,
                height,
                self.config.border_width,
                self.config.border_color,
            );
        }

        // Calculate text area
        let text_x = pos.x + self.config.padding;
        let mut text_y = pos.y + self.config.padding;
        let text_width = width - self.config.padding * 2.0;

        // Draw title if present
        if let Some(ref title) = self.title {
            let title_size = self.config.font_size + 4.0;
            self.draw_line_with_alignment(
                title,
                text_x,
                text_y,
                text_width,
                title_size,
                Color::new(1.0, 1.0, 0.0, 1.0),
            );
            text_y += title_size + self.config.line_spacing * 2.0;
        }

        // Draw wrapped text lines
        for line in &self.lines {
            let wrapped_lines = self.wrap_text(line, text_width);

            for wrapped_line in wrapped_lines {
                // Check if line fits in panel
                if text_y + self.config.font_size > pos.y + height - self.config.padding {
                    self.draw_line_with_alignment(
                        "...",
                        text_x,
                        text_y,
                        text_width,
                        self.config.font_size,
                        self.config.text_color,
                    );
                    return;
                }

                self.draw_line_with_alignment(
                    &wrapped_line,
                    text_x,
                    text_y,
                    text_width,
                    self.config.font_size,
                    self.config.text_color,
                );

                text_y += self.config.font_size + self.config.line_spacing;
            }
        }
    }

    /// Wrap text to fit within width
    fn wrap_text(&self, text: &str, width: f32) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let dims = measure_text(&test_line, None, self.config.font_size as u16, 1.0);

            if dims.width <= width {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    // === Utility ===

    pub fn contains_point(&self, point: Vec2) -> bool {
        let pos = self.config.position;
        point.x >= pos.x
            && point.x <= pos.x + self.config.width
            && point.y >= pos.y
            && point.y <= pos.y + self.config.height
    }
}

impl Default for TextPanel {
    fn default() -> Self {
        let config = TextPanelConfig::default();
        Self::new(Vec2::new(config.position.x, config.position.y), Vec2::new(config.width, config.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_panel_creation() {
        let panel = TextPanel::default();
        assert!(panel.is_visible());
        assert_eq!(panel.line_count(), 0);
    }

    #[test]
    fn test_text_panel_content() {
        let mut panel = TextPanel::default();
        panel.set_text("Line 1\nLine 2\nLine 3");
        assert_eq!(panel.line_count(), 3);
    }

    #[test]
    fn test_text_panel_with_title() {
        let panel = TextPanel::default()
            .with_title("Test Title".to_string());
        assert!(panel.title.is_some());
    }

    #[test]
    fn test_text_panel_visibility() {
        let mut panel = TextPanel::default();
        assert!(panel.is_visible());

        panel.set_visible(false);
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_contains_point() {
        let config = TextPanelConfig {
            position: Vec2::new(100.0, 100.0),
            width: 200.0,
            height: 150.0,
            ..Default::default()
        };
        let panel = TextPanel::new(config);

        assert!(panel.contains_point(Vec2::new(150.0, 150.0)));
        assert!(!panel.contains_point(Vec2::new(50.0, 50.0)));
        assert!(!panel.contains_point(Vec2::new(350.0, 150.0)));
    }
}
