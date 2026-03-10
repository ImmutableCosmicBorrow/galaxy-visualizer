use eframe::egui;
use orchestrator::ui::UiToOrchestratorCommand;

use crate::comms::OrchestratorComms;
use crate::models::SpawnStage;
use crate::state::UiState;

/// Render the top control bar (game-mode, pause/resume, create planet, etc.).
pub fn show_top_panel(ctx: &egui::Context, ui_state: &mut UiState, comms: &OrchestratorComms) {
    egui::TopBottomPanel::top("top_controls").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Left side buttons
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button("Switch Game Mode").clicked() {
                    comms.send_expect(
                        UiToOrchestratorCommand::SwitchGameMode,
                        "Failed to send SwitchGameMode command",
                    );
                }
                if ui.button("End Game").clicked() {
                    comms.send_expect(
                        UiToOrchestratorCommand::EndGame,
                        "Failed to send EndGame command",
                    );
                }
                if ui.button("Pause Game").clicked() {
                    comms.send_expect(
                        UiToOrchestratorCommand::PauseGame,
                        "Failed to send PauseGame command",
                    );
                }
                if ui.button("Resume Game").clicked() {
                    comms.send_expect(
                        UiToOrchestratorCommand::ResumeGame,
                        "Failed to send ResumeGame command",
                    );
                }
            });

            // Right side button
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Create Planet").clicked() {
                    ui_state.pending_spawn_pos =
                        Some(ui.max_rect().right_top() + egui::vec2(-10.0, 30.0));
                    ui_state.spawn_stage = SpawnStage::SelectingType;
                    ui_state.selected_neighbors.clear();
                }
            });
        });
    });
}
