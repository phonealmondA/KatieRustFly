mod map_data;

use macroquad::prelude::*;
use map_data::*;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    MainMenu,
    MapEditor,
}

#[derive(Debug, Clone, PartialEq)]
enum EditField {
    MapName,
    MapDescription,
    BodyName(usize),
    BodyMass(usize),
    BodyRadius(usize),
    BodyColorR(usize),
    BodyColorG(usize),
    BodyColorB(usize),
    BodyOrbitalDistance(usize),
    BodyOrbitalPeriod(usize),
    BodyInitialAngle(usize),
    None,
}

#[derive(Debug)]
struct App {
    state: AppState,
    current_map: MapConfiguration,
    maps_folder: String,
    active_field: EditField,
    text_buffer: String,
    selected_body: Option<usize>,
    scroll_offset: f32,
}

impl App {
    fn new() -> Self {
        // Get the directory where the executable is located
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let maps_folder = exe_dir.join("maps");

        App {
            state: AppState::MainMenu,
            current_map: MapConfiguration::new_empty(),
            maps_folder: maps_folder.to_string_lossy().to_string(),
            active_field: EditField::None,
            text_buffer: String::new(),
            selected_body: None,
            scroll_offset: 0.0,
        }
    }

    fn start_editing_field(&mut self, field: EditField) {
        self.active_field = field.clone();
        self.text_buffer = match &field {
            EditField::MapName => self.current_map.name.clone(),
            EditField::MapDescription => self.current_map.description.clone(),
            EditField::BodyName(i) => self.current_map.celestial_bodies[*i].name.clone(),
            EditField::BodyMass(i) => self.current_map.celestial_bodies[*i].mass.to_string(),
            EditField::BodyRadius(i) => self.current_map.celestial_bodies[*i].radius.to_string(),
            EditField::BodyColorR(i) => self.current_map.celestial_bodies[*i].color.r.to_string(),
            EditField::BodyColorG(i) => self.current_map.celestial_bodies[*i].color.g.to_string(),
            EditField::BodyColorB(i) => self.current_map.celestial_bodies[*i].color.b.to_string(),
            EditField::BodyOrbitalDistance(i) => {
                self.current_map.celestial_bodies[*i]
                    .orbital_distance
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "0".to_string())
            }
            EditField::BodyOrbitalPeriod(i) => {
                self.current_map.celestial_bodies[*i]
                    .orbital_period
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "0".to_string())
            }
            EditField::BodyInitialAngle(i) => {
                self.current_map.celestial_bodies[*i].initial_angle.to_string()
            }
            EditField::None => String::new(),
        };
    }

    fn finish_editing(&mut self) {
        let field = self.active_field.clone();
        match field {
            EditField::MapName => {
                self.current_map.name = self.text_buffer.clone();
            }
            EditField::MapDescription => {
                self.current_map.description = self.text_buffer.clone();
            }
            EditField::BodyName(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    self.current_map.celestial_bodies[i].name = self.text_buffer.clone();
                }
            }
            EditField::BodyMass(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].mass = val.max(1.0);
                    }
                }
            }
            EditField::BodyRadius(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].radius = val.max(100.0);
                    }
                }
            }
            EditField::BodyColorR(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].color.r = val.max(0.0).min(1.0);
                    }
                }
            }
            EditField::BodyColorG(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].color.g = val.max(0.0).min(1.0);
                    }
                }
            }
            EditField::BodyColorB(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].color.b = val.max(0.0).min(1.0);
                    }
                }
            }
            EditField::BodyOrbitalDistance(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].orbital_distance = if val > 0.0 {
                            Some(val)
                        } else {
                            None
                        };
                    }
                }
            }
            EditField::BodyOrbitalPeriod(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].orbital_period = if val > 0.0 {
                            Some(val)
                        } else {
                            None
                        };
                    }
                }
            }
            EditField::BodyInitialAngle(i) => {
                if i < self.current_map.celestial_bodies.len() {
                    if let Ok(val) = self.text_buffer.parse::<f32>() {
                        self.current_map.celestial_bodies[i].initial_angle = val;
                    }
                }
            }
            EditField::None => {}
        }
        self.active_field = EditField::None;
        self.text_buffer.clear();
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Katie Map Maker".to_owned(),
        window_width: 1280,
        window_height: 800,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();

    // Ensure maps folder exists
    let _ = std::fs::create_dir_all(&app.maps_folder);

    loop {
        clear_background(Color::from_rgba(20, 20, 30, 255));

        match app.state {
            AppState::MainMenu => {
                if let Some(new_state) = draw_main_menu(&mut app).await {
                    app.state = new_state;
                }
            }
            AppState::MapEditor => {
                if let Some(new_state) = draw_map_editor(&mut app).await {
                    app.state = new_state;
                }
            }
        }

        next_frame().await
    }
}

async fn draw_main_menu(app: &mut App) -> Option<AppState> {
    let screen_width = screen_width();
    let screen_height = screen_height();

    // Title
    let title = "Katie Map Maker";
    let title_size = 60.0;
    let title_dims = measure_text(title, None, title_size as u16, 1.0);
    draw_text(
        title,
        screen_width / 2.0 - title_dims.width / 2.0,
        120.0,
        title_size,
        WHITE,
    );

    // Subtitle
    let subtitle = "Create custom maps for KatieFlySimRust";
    let subtitle_size = 24.0;
    let subtitle_dims = measure_text(subtitle, None, subtitle_size as u16, 1.0);
    draw_text(
        subtitle,
        screen_width / 2.0 - subtitle_dims.width / 2.0,
        160.0,
        subtitle_size,
        Color::from_rgba(150, 150, 150, 255),
    );

    // Buttons
    let button_width = 300.0;
    let button_height = 60.0;
    let button_x = screen_width / 2.0 - button_width / 2.0;
    let mut button_y = 250.0;
    let button_spacing = 80.0;

    // New Map button
    if draw_button("New Map", button_x, button_y, button_width, button_height) {
        app.current_map = MapConfiguration::new_empty();
        app.selected_body = None;
        return Some(AppState::MapEditor);
    }
    button_y += button_spacing;

    // Load Map button (TODO: implement)
    if draw_button("Load Map", button_x, button_y, button_width, button_height) {
        app.current_map = MapConfiguration::new_empty();
        app.selected_body = None;
        return Some(AppState::MapEditor);
    }
    button_y += button_spacing;

    // Exit button
    if draw_button("Exit", button_x, button_y, button_width, button_height) {
        std::process::exit(0);
    }

    // Instructions at bottom
    let instructions = "Maps will be saved to: ./maps/ (same folder as this executable)";
    let inst_size = 18.0;
    let inst_dims = measure_text(instructions, None, inst_size as u16, 1.0);
    draw_text(
        instructions,
        screen_width / 2.0 - inst_dims.width / 2.0,
        screen_height - 30.0,
        inst_size,
        Color::from_rgba(100, 100, 100, 255),
    );

    None
}

async fn draw_map_editor(app: &mut App) -> Option<AppState> {
    let screen_width = screen_width();
    let screen_height = screen_height();

    // Handle text input
    handle_text_input(app);

    // Title
    draw_text("Map Editor", 20.0, 40.0, 40.0, WHITE);

    // Back button
    if draw_button("← Back", 20.0, screen_height - 80.0, 150.0, 50.0) {
        return Some(AppState::MainMenu);
    }

    // Save button
    if draw_button("💾 Save Map", screen_width - 220.0, screen_height - 80.0, 200.0, 50.0) {
        match app.current_map.validate() {
            Ok(_) => {
                let filename = format!("{}/{}.ron", app.maps_folder, app.current_map.name);
                match app.current_map.save_to_file(&filename) {
                    Ok(_) => println!("✓ Map saved: {}", filename),
                    Err(e) => println!("✗ Save failed: {}", e),
                }
            }
            Err(e) => println!("✗ Validation failed: {}", e),
        }
    }

    // Two-column layout
    let left_panel_width = screen_width * 0.35;
    let right_panel_x = left_panel_width + 20.0;

    // === LEFT PANEL: Map Settings & Body List ===
    draw_left_panel(app, left_panel_width);

    // === RIGHT PANEL: Body Editor ===
    draw_right_panel(app, right_panel_x, screen_width - right_panel_x - 20.0);

    None
}

fn draw_left_panel(app: &mut App, panel_width: f32) {
    let mut y = 80.0;

    // Map Name
    draw_text("Map Name:", 20.0, y, 24.0, WHITE);
    y += 30.0;
    if draw_text_field(
        &app.current_map.name,
        20.0,
        y,
        panel_width - 40.0,
        35.0,
        &app.active_field,
        &EditField::MapName,
        &app.text_buffer,
    ) {
        app.start_editing_field(EditField::MapName);
    }
    y += 50.0;

    // Map Description
    draw_text("Description:", 20.0, y, 24.0, WHITE);
    y += 30.0;
    if draw_text_field(
        &app.current_map.description,
        20.0,
        y,
        panel_width - 40.0,
        35.0,
        &app.active_field,
        &EditField::MapDescription,
        &app.text_buffer,
    ) {
        app.start_editing_field(EditField::MapDescription);
    }
    y += 60.0;

    // Celestial Bodies List
    draw_text(
        &format!("Celestial Bodies ({})", app.current_map.celestial_bodies.len()),
        20.0,
        y,
        28.0,
        WHITE,
    );
    y += 40.0;

    // Body list
    for (i, body) in app.current_map.celestial_bodies.iter().enumerate() {
        let is_selected = app.selected_body == Some(i);
        let button_color = if is_selected {
            Color::from_rgba(70, 130, 180, 255)
        } else {
            Color::from_rgba(50, 50, 70, 255)
        };

        // Draw body button
        draw_rectangle(20.0, y, panel_width - 40.0, 50.0, button_color);
        draw_rectangle_lines(20.0, y, panel_width - 40.0, 50.0, 2.0, WHITE);

        // Body color indicator
        let body_color = Color::new(body.color.r, body.color.g, body.color.b, 1.0);
        draw_circle(40.0, y + 25.0, 12.0, body_color);
        draw_circle_lines(40.0, y + 25.0, 12.0, 2.0, WHITE);

        // Body name and info
        draw_text(&body.name, 65.0, y + 20.0, 20.0, WHITE);
        draw_text(
            &format!("R:{:.0} M:{:.0}", body.radius, body.mass),
            65.0,
            y + 38.0,
            16.0,
            Color::from_rgba(180, 180, 180, 255),
        );

        // Delete button
        if draw_small_button("×", panel_width - 50.0, y + 10.0, 30.0, 30.0) {
            app.current_map.celestial_bodies.remove(i);
            if app.selected_body == Some(i) {
                app.selected_body = None;
            } else if let Some(sel) = app.selected_body {
                if sel > i {
                    app.selected_body = Some(sel - 1);
                }
            }
            return;
        }

        // Click to select
        let mouse_pos = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left)
            && mouse_pos.0 >= 20.0
            && mouse_pos.0 <= panel_width - 40.0
            && mouse_pos.1 >= y
            && mouse_pos.1 <= y + 50.0
        {
            app.selected_body = Some(i);
        }

        y += 60.0;
    }

    // Add Body button
    if draw_button("+ Add Body", 20.0, y + 10.0, panel_width - 40.0, 50.0) {
        let new_body = CelestialBodyConfig::new_planet(&format!(
            "Body {}",
            app.current_map.celestial_bodies.len() + 1
        ));
        app.current_map.celestial_bodies.push(new_body);
        app.selected_body = Some(app.current_map.celestial_bodies.len() - 1);
    }
}

fn draw_right_panel(app: &mut App, x: f32, width: f32) {
    if let Some(body_idx) = app.selected_body {
        if body_idx >= app.current_map.celestial_bodies.len() {
            return;
        }

        let mut y = 80.0;

        // Clone values we need to avoid borrow checker issues
        let body_name = app.current_map.celestial_bodies[body_idx].name.clone();
        let body_mass = app.current_map.celestial_bodies[body_idx].mass;
        let body_radius = app.current_map.celestial_bodies[body_idx].radius;
        let body_color = app.current_map.celestial_bodies[body_idx].color.clone();
        let body_is_pinned = app.current_map.celestial_bodies[body_idx].is_pinned;
        let body_orbital_parent = app.current_map.celestial_bodies[body_idx].orbital_parent_index;
        let body_orbital_distance = app.current_map.celestial_bodies[body_idx].orbital_distance;
        let body_orbital_period = app.current_map.celestial_bodies[body_idx].orbital_period;
        let body_initial_angle = app.current_map.celestial_bodies[body_idx].initial_angle;

        draw_text(
            &format!("Editing: {}", body_name),
            x,
            y,
            32.0,
            Color::from_rgba(100, 200, 255, 255),
        );
        y += 50.0;

        // Body Name
        draw_text("Name:", x, y, 22.0, WHITE);
        y += 28.0;
        if draw_text_field(
            &body_name,
            x,
            y,
            width - 20.0,
            35.0,
            &app.active_field,
            &EditField::BodyName(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyName(body_idx));
        }
        y += 50.0;

        // Mass
        draw_text("Mass:", x, y, 22.0, WHITE);
        y += 28.0;
        if draw_text_field(
            &body_mass.to_string(),
            x,
            y,
            width - 20.0,
            35.0,
            &app.active_field,
            &EditField::BodyMass(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyMass(body_idx));
        }
        y += 50.0;

        // Radius
        draw_text("Radius:", x, y, 22.0, WHITE);
        y += 28.0;
        if draw_text_field(
            &body_radius.to_string(),
            x,
            y,
            width - 20.0,
            35.0,
            &app.active_field,
            &EditField::BodyRadius(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyRadius(body_idx));
        }
        y += 50.0;

        // Color
        draw_text("Color (RGB, 0.0-1.0):", x, y, 22.0, WHITE);
        y += 30.0;

        // Color preview
        let color = Color::new(body_color.r, body_color.g, body_color.b, 1.0);
        draw_circle(x + width / 2.0, y + 20.0, 25.0, color);
        draw_circle_lines(x + width / 2.0, y + 20.0, 25.0, 2.0, WHITE);
        y += 60.0;

        // R
        draw_text("R:", x, y, 20.0, RED);
        if draw_text_field(
            &format!("{:.2}", body_color.r),
            x + 30.0,
            y,
            100.0,
            30.0,
            &app.active_field,
            &EditField::BodyColorR(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyColorR(body_idx));
        }
        y += 40.0;

        // G
        draw_text("G:", x, y, 20.0, GREEN);
        if draw_text_field(
            &format!("{:.2}", body_color.g),
            x + 30.0,
            y,
            100.0,
            30.0,
            &app.active_field,
            &EditField::BodyColorG(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyColorG(body_idx));
        }
        y += 40.0;

        // B
        draw_text("B:", x, y, 20.0, BLUE);
        if draw_text_field(
            &format!("{:.2}", body_color.b),
            x + 30.0,
            y,
            100.0,
            30.0,
            &app.active_field,
            &EditField::BodyColorB(body_idx),
        &app.text_buffer,
        ) {
            app.start_editing_field(EditField::BodyColorB(body_idx));
        }
        y += 50.0;

        // Orbital Settings
        draw_text("Orbital Settings:", x, y, 24.0, Color::from_rgba(255, 200, 100, 255));
        y += 35.0;

        // Is Pinned toggle
        if draw_checkbox("Pinned (stationary)", body_is_pinned, x, y) {
            app.current_map.celestial_bodies[body_idx].is_pinned = !body_is_pinned;
        }
        y += 40.0;

        if !body_is_pinned {
            // Orbital Parent
            draw_text("Orbital Parent:", x, y, 20.0, WHITE);
            y += 25.0;
            let parent_text = match body_orbital_parent {
                Some(idx) => {
                    if idx < app.current_map.celestial_bodies.len() {
                        app.current_map.celestial_bodies[idx].name.clone()
                    } else {
                        "Invalid".to_string()
                    }
                }
                None => "None".to_string(),
            };

            // Cycle parent button
            if draw_button(&parent_text, x, y, width - 20.0, 35.0) {
                let current_parent = app.current_map.celestial_bodies[body_idx].orbital_parent_index;
                let next_parent = match current_parent {
                    None => {
                        if app.current_map.celestial_bodies.len() > 1 {
                            Some(if body_idx == 0 { 1 } else { 0 })
                        } else {
                            None
                        }
                    }
                    Some(idx) => {
                        let mut next = (idx + 1) % app.current_map.celestial_bodies.len();
                        if next == body_idx {
                            next = (next + 1) % app.current_map.celestial_bodies.len();
                        }
                        if app.current_map.celestial_bodies.len() <= 1 || next == idx {
                            None
                        } else {
                            Some(next)
                        }
                    }
                };
                app.current_map.celestial_bodies[body_idx].orbital_parent_index = next_parent;
            }
            y += 45.0;

            // Orbital Distance
            draw_text("Orbital Distance:", x, y, 20.0, WHITE);
            y += 25.0;
            let dist_text = body_orbital_distance
                .map(|d| d.to_string())
                .unwrap_or_else(|| "0".to_string());
            if draw_text_field(
                &dist_text,
                x,
                y,
                width - 20.0,
                35.0,
                &app.active_field,
                &EditField::BodyOrbitalDistance(body_idx),
        &app.text_buffer,
            ) {
                app.start_editing_field(EditField::BodyOrbitalDistance(body_idx));
            }
            y += 45.0;

            // Orbital Period
            draw_text("Orbital Period (seconds):", x, y, 20.0, WHITE);
            y += 25.0;
            let period_text = body_orbital_period
                .map(|p| p.to_string())
                .unwrap_or_else(|| "0".to_string());
            if draw_text_field(
                &period_text,
                x,
                y,
                width - 20.0,
                35.0,
                &app.active_field,
                &EditField::BodyOrbitalPeriod(body_idx),
        &app.text_buffer,
            ) {
                app.start_editing_field(EditField::BodyOrbitalPeriod(body_idx));
            }
            y += 45.0;

            // Initial Angle
            draw_text("Initial Angle (radians):", x, y, 20.0, WHITE);
            y += 25.0;
            if draw_text_field(
                &body_initial_angle.to_string(),
                x,
                y,
                width - 20.0,
                35.0,
                &app.active_field,
                &EditField::BodyInitialAngle(body_idx),
        &app.text_buffer,
            ) {
                app.start_editing_field(EditField::BodyInitialAngle(body_idx));
            }
        }
    } else {
        // No body selected
        draw_text(
            "← Select a body to edit",
            x + 20.0,
            screen_height() / 2.0,
            24.0,
            Color::from_rgba(150, 150, 150, 255),
        );
    }
}

fn handle_text_input(app: &mut App) {
    if app.active_field == EditField::None {
        return;
    }

    // Handle character input
    if let Some(ch) = get_char_pressed() {
        if ch.is_ascii() && !ch.is_control() {
            app.text_buffer.push(ch);
        }
    }

    // Handle backspace
    if is_key_pressed(KeyCode::Backspace) {
        app.text_buffer.pop();
    }

    // Handle enter or escape to finish editing
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
        app.finish_editing();
    }
}

fn draw_text_field(
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    active_field: &EditField,
    field_id: &EditField,
    text_buffer: &str,
) -> bool {
    let is_active = active_field == field_id;
    let mouse_pos = mouse_position();
    let is_hovered = mouse_pos.0 >= x
        && mouse_pos.0 <= x + width
        && mouse_pos.1 >= y
        && mouse_pos.1 <= y + height;

    let bg_color = if is_active {
        Color::from_rgba(60, 60, 80, 255)
    } else if is_hovered {
        Color::from_rgba(45, 45, 60, 255)
    } else {
        Color::from_rgba(35, 35, 45, 255)
    };

    draw_rectangle(x, y, width, height, bg_color);
    draw_rectangle_lines(
        x,
        y,
        width,
        height,
        2.0,
        if is_active {
            Color::from_rgba(100, 200, 255, 255)
        } else {
            Color::from_rgba(100, 100, 120, 255)
        },
    );

    // Show buffer when editing, otherwise show original text
    let display_text = if is_active { text_buffer } else { text };
    draw_text(display_text, x + 10.0, y + height - 10.0, 20.0, WHITE);

    // Draw cursor if active
    if is_active {
        let cursor_x = x + 10.0 + measure_text(display_text, None, 20, 1.0).width;
        draw_line(
            cursor_x,
            y + 5.0,
            cursor_x,
            y + height - 5.0,
            2.0,
            WHITE,
        );
    }

    is_hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_button(text: &str, x: f32, y: f32, width: f32, height: f32) -> bool {
    let mouse_pos = mouse_position();
    let is_hovered = mouse_pos.0 >= x
        && mouse_pos.0 <= x + width
        && mouse_pos.1 >= y
        && mouse_pos.1 <= y + height;

    let button_color = if is_hovered {
        Color::from_rgba(70, 130, 180, 255)
    } else {
        Color::from_rgba(50, 50, 70, 255)
    };

    draw_rectangle(x, y, width, height, button_color);
    draw_rectangle_lines(x, y, width, height, 2.0, WHITE);

    let text_size = 24.0;
    let text_dims = measure_text(text, None, text_size as u16, 1.0);
    draw_text(
        text,
        x + width / 2.0 - text_dims.width / 2.0,
        y + height / 2.0 + text_dims.height / 2.0,
        text_size,
        WHITE,
    );

    is_hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_small_button(text: &str, x: f32, y: f32, width: f32, height: f32) -> bool {
    let mouse_pos = mouse_position();
    let is_hovered = mouse_pos.0 >= x
        && mouse_pos.0 <= x + width
        && mouse_pos.1 >= y
        && mouse_pos.1 <= y + height;

    let button_color = if is_hovered {
        Color::from_rgba(200, 50, 50, 255)
    } else {
        Color::from_rgba(120, 40, 40, 255)
    };

    draw_rectangle(x, y, width, height, button_color);
    draw_rectangle_lines(x, y, width, height, 2.0, WHITE);

    let text_size = 24.0;
    let text_dims = measure_text(text, None, text_size as u16, 1.0);
    draw_text(
        text,
        x + width / 2.0 - text_dims.width / 2.0,
        y + height / 2.0 + text_dims.height / 2.0,
        text_size,
        WHITE,
    );

    is_hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_checkbox(label: &str, checked: bool, x: f32, y: f32) -> bool {
    let box_size = 24.0;
    let mouse_pos = mouse_position();
    let is_hovered = mouse_pos.0 >= x
        && mouse_pos.0 <= x + box_size + 200.0
        && mouse_pos.1 >= y
        && mouse_pos.1 <= y + box_size;

    // Draw checkbox
    draw_rectangle(
        x,
        y,
        box_size,
        box_size,
        if is_hovered {
            Color::from_rgba(60, 60, 80, 255)
        } else {
            Color::from_rgba(40, 40, 50, 255)
        },
    );
    draw_rectangle_lines(x, y, box_size, box_size, 2.0, WHITE);

    // Draw checkmark if checked
    if checked {
        draw_line(x + 5.0, y + 12.0, x + 10.0, y + 18.0, 3.0, GREEN);
        draw_line(x + 10.0, y + 18.0, x + 19.0, y + 6.0, 3.0, GREEN);
    }

    // Draw label
    draw_text(label, x + box_size + 10.0, y + 18.0, 20.0, WHITE);

    is_hovered && is_mouse_button_pressed(MouseButton::Left)
}
