use eframe::egui;
use orchestrator::ExplorerType;

use crate::state::StartupState;

pub struct StartupMenuResult {
    pub start_requested: bool,
}

pub fn show_startup_menu(ctx: &egui::Context, state: &mut StartupState) -> StartupMenuResult {
    let mut start_requested = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Start Game");
        });
        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label("Explorer selection");
            ui.add_space(4.0);

            explorer_combo(ui, "Explorer slot 1 (required)", &mut state.explorer_slot_one, false);
            explorer_combo(ui, "Explorer slot 2 (optional)", &mut state.explorer_slot_two, true);
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label("Game step");
            ui.add_space(4.0);
            ui.add(
                egui::Slider::new(&mut state.game_step_ms, 2000..=10000)
                    .text("Step (ms)")
                    .clamping(egui::SliderClamping::Always),
            );
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label("Galaxy file");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut state.galaxy_path);
                if ui.button("Load").clicked() {
                    state.load_galaxy_file();
                }
                if ui.button("Save").clicked() {
                    state.save_galaxy_file();
                }
            });

            ui.add_space(6.0);

            let editor = egui::TextEdit::multiline(&mut state.galaxy_contents)
                .desired_rows(16)
                .lock_focus(true)
                .code_editor();
            if ui.add(editor).changed() {
                state.galaxy_dirty = true;
            }

            if state.galaxy_dirty {
                ui.label("Unsaved changes");
            }

            if let Some(msg) = &state.last_file_status {
                ui.label(msg);
            }
            if let Some(err) = &state.last_file_error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        ui.add_space(12.0);

        let can_start = state.explorer_slot_one.is_some();
        if ui
            .add_enabled(can_start, egui::Button::new("Start Game"))
            .clicked()
        {
            start_requested = true;
        }
    });

    StartupMenuResult { start_requested }
}

fn explorer_combo(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ExplorerType>,
    allow_none: bool,
) {
    let selected_text = match value {
        Some(ExplorerType::Explorer) => "Nico Explorer",
        Some(ExplorerType::Vojager) => "Vojager",
        Some(ExplorerType::Nomad) => "Nomad",
        None => "None",
    };

    egui::ComboBox::from_label(label)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            if allow_none {
                if ui.selectable_label(value.is_none(), "None").clicked() {
                    *value = None;
                }
            }

            if ui
                .selectable_label(matches!(value, Some(ExplorerType::Explorer)), "Nico Explorer")
                .clicked()
            {
                *value = Some(ExplorerType::Explorer);
            }
            if ui
                .selectable_label(matches!(value, Some(ExplorerType::Vojager)), "Vojager")
                .clicked()
            {
                *value = Some(ExplorerType::Vojager);
            }
            if ui
                .selectable_label(matches!(value, Some(ExplorerType::Nomad)), "Nomad")
                .clicked()
            {
                *value = Some(ExplorerType::Nomad);
            }
        });
}
