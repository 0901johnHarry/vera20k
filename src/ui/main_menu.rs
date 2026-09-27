//! Shared skirmish settings, startup errors and development loading presentation.
//! Normal match setup belongs to the retail shell in `skirmish_shell`.

use crate::ui::client_theme;

/// Player's chosen faction side for skirmish games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkirmishSide {
    #[default]
    Allied,
    Soviet,
}

impl SkirmishSide {
    pub fn label(self) -> &'static str {
        match self {
            Self::Allied => "Allied",
            Self::Soviet => "Soviet",
        }
    }

    pub const ALL: [SkirmishSide; 2] = [Self::Allied, Self::Soviet];
}

/// Individual country selection for skirmish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkirmishCountry {
    #[default]
    America,
    Korea,
    France,
    Germany,
    GreatBritain,
    Libya,
    Iraq,
    Cuba,
    Russia,
    Yuri,
}

impl SkirmishCountry {
    pub const ALL: [SkirmishCountry; 10] = [
        Self::America,
        Self::Korea,
        Self::France,
        Self::Germany,
        Self::GreatBritain,
        Self::Libya,
        Self::Iraq,
        Self::Cuba,
        Self::Russia,
        Self::Yuri,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::America => "America",
            Self::Korea => "Korea",
            Self::France => "France",
            Self::Germany => "Germany",
            Self::GreatBritain => "Great Britain",
            Self::Libya => "Libya",
            Self::Iraq => "Iraq",
            Self::Cuba => "Cuba",
            Self::Russia => "Russia",
            Self::Yuri => "Yuri",
        }
    }

    pub fn side(self) -> SkirmishSide {
        match self {
            Self::America | Self::Korea | Self::France | Self::Germany | Self::GreatBritain => {
                SkirmishSide::Allied
            }
            Self::Libya | Self::Iraq | Self::Cuba | Self::Russia | Self::Yuri => {
                SkirmishSide::Soviet
            }
        }
    }

    pub fn country_name(self) -> &'static str {
        match self {
            Self::America => "Americans",
            Self::Korea => "Alliance",
            Self::France => "French",
            Self::Germany => "Germans",
            Self::GreatBritain => "British",
            Self::Libya => "Africans",
            Self::Iraq => "Arabs",
            Self::Cuba => "Confederation",
            Self::Russia => "Russians",
            Self::Yuri => "YuriCountry",
        }
    }
}

/// Player's chosen start position on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPosition {
    /// Automatic; let the game route through spawn picking.
    Auto,
    /// Specific waypoint index.
    Position(u8),
}

impl Default for StartPosition {
    fn default() -> Self {
        StartPosition::Position(0)
    }
}

/// All configurable skirmish options, set in the main menu before launch.
#[derive(Debug, Clone)]
pub struct SkirmishSettings {
    pub selected_map_idx: usize,
    pub player_country: SkirmishCountry,
    pub ai_country: SkirmishCountry,
    pub starting_credits: i32,
    pub start_position: StartPosition,
    pub short_game: bool,
    /// Allow mouse-wheel zoom in-game (zoom in/out the battlefield).
    pub zoom_enabled: bool,
}

impl Default for SkirmishSettings {
    fn default() -> Self {
        Self {
            selected_map_idx: 0,
            player_country: SkirmishCountry::default(),
            ai_country: SkirmishCountry::Russia,
            starting_credits: 10_000,
            start_position: StartPosition::default(),
            short_game: true,
            zoom_enabled: true,
        }
    }
}

/// Display an actionable startup failure. This surface cannot launch a match.
/// Returns true when the player chooses to quit.
pub(crate) fn draw_shell_error(
    ctx: &egui::Context,
    error: &str,
    asset_root: Option<&std::path::Path>,
) -> bool {
    let palette = client_theme::apply_client_theme(ctx);
    let mut quit = false;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.bg).inner_margin(24.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Unable to load the game menu");
                ui.add_space(12.0);
                ui.label("VERA20k needs the original Red Alert 2: Yuri's Revenge game files.");
                if let Some(root) = asset_root {
                    ui.add_space(12.0);
                    ui.strong("Game files searched in:");
                    ui.label(root.display().to_string());
                }
                ui.add_space(12.0);
                ui.label(error);
                ui.add_space(12.0);
                ui.label("Set [paths] ra2_dir in config.toml to your game installation, then restart. The configuration can be beside the executable or in the folder you launch from.");
                ui.add_space(20.0);
                quit = ui.button("Quit").clicked();
            });
        });
    quit
}

/// Draw the loading screen shown while map data is being parsed.
pub fn draw_loading_screen(ctx: &egui::Context, map_name: &str) {
    let palette = client_theme::apply_client_theme(ctx);

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.bg))
        .show(ctx, |ui| {
            client_theme::paint_background(ui, palette);
            let panel = ui.max_rect();

            if let Some(texture) = loading_screen_texture(ctx) {
                let image_size = texture.size_vec2();
                let scale = (panel.width() / image_size.x).min(panel.height() / image_size.y);
                let desired = image_size * (scale * 0.96);
                let image_rect = egui::Align2::CENTER_CENTER.align_size_within_rect(desired, panel);
                ui.put(image_rect, egui::Image::new((texture.id(), desired)));
                ui.painter().rect_filled(
                    image_rect.expand(2.0),
                    20.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 36),
                );
            }

            let overlay_size = egui::vec2(430.0, 132.0);
            let overlay_rect = egui::Rect::from_min_size(
                egui::pos2(panel.left() + 28.0, panel.bottom() - overlay_size.y - 28.0),
                overlay_size,
            );
            ui.painter().rect_filled(
                overlay_rect,
                18.0,
                egui::Color32::from_rgba_premultiplied(9, 14, 20, 214),
            );
            ui.painter().rect_stroke(
                overlay_rect,
                18.0,
                egui::Stroke::new(1.0, palette.line.gamma_multiply(0.85)),
                egui::StrokeKind::Middle,
            );

            let text_origin = overlay_rect.min + egui::vec2(18.0, 16.0);
            ui.painter().text(
                text_origin,
                egui::Align2::LEFT_TOP,
                "Mission deployment",
                egui::FontId::proportional(14.0),
                palette.accent,
            );
            ui.painter().text(
                text_origin + egui::vec2(0.0, 24.0),
                egui::Align2::LEFT_TOP,
                "Loading...",
                egui::FontId::proportional(32.0),
                palette.text,
            );
            ui.painter().text(
                text_origin + egui::vec2(0.0, 68.0),
                egui::Align2::LEFT_TOP,
                format!("Map: {}", map_name),
                egui::FontId::proportional(16.0),
                palette.text_muted,
            );
            ui.painter().text(
                text_origin + egui::vec2(0.0, 92.0),
                egui::Align2::LEFT_TOP,
                "Parsing map, spawning actors, and preparing assets.",
                egui::FontId::proportional(14.0),
                palette.text_muted,
            );
        });
}

fn loading_screen_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let texture_id = egui::Id::new("loading_screen_texture");
    if let Some(existing) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(texture_id)) {
        return Some(existing);
    }

    let image = loading_screen_image()?;
    let texture = ctx.load_texture(
        "loading_screen_texture",
        image.clone(),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(texture_id, texture.clone()));
    Some(texture)
}

fn loading_screen_image() -> Option<&'static egui::ColorImage> {
    // Loading screen image removed — the caller gracefully handles None.
    None
}
