use common_game::logging::Channel;
use eframe::egui;
use orchestrator::ExplorerType;
use orchestrator::logging_utils::LogTarget;
use orchestrator::payload;
use orchestrator::ui::UiToOrchestratorCommand;

use crate::comms::OrchestratorComms;
use crate::state::{AnimationState, ExplorerState, GalaxyState, PollingTimers, UiState};
use crate::ui;
use crate::update_handler;

/// Top-level application struct – composed entirely of focused sub-states.
pub struct GalaxyApp {
    pub galaxy_state: GalaxyState,
    pub explorer_state: ExplorerState,
    pub ui_state: UiState,
    pub animation_state: AnimationState,
    pub timers: PollingTimers,
    pub comms: OrchestratorComms,
}

impl GalaxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let game_step = 3000;
        let (mut orch, cmd_sender, update_receiver) = orchestrator::create_with_path(
            "galaxy/test_galaxy.txt",
            ExplorerType::Explorer,
            Some(ExplorerType::Vojager),
            None,
            game_step,
        );

        cmd_sender
            .send(UiToOrchestratorCommand::GetGalaxy)
            .expect("Failed to send initial GetGalaxy command");

        cmd_sender
            .send(UiToOrchestratorCommand::GetExplorersPosition)
            .expect("Failed to send initial GetExplorerPosition command");

        std::thread::spawn(move || {
            orch.run();
            orchestrator::logging_utils::log_internal(
                LogTarget::General,
                Channel::Info,
                payload!(
                    message : "Orchestrator created",
                ),
            );

            let tick = std::time::Duration::from_millis(16);
            loop {
                std::thread::sleep(tick);
            }
        });

        Self {
            galaxy_state: GalaxyState::new(),
            explorer_state: ExplorerState::new(),
            ui_state: UiState::new(),
            animation_state: AnimationState::new(),
            timers: PollingTimers::new(game_step),
            comms: OrchestratorComms::new(cmd_sender, update_receiver),
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App – the main loop, now just thin orchestration
// ---------------------------------------------------------------------------

impl eframe::App for GalaxyApp {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Handle window close (X button) ──────────────────────────────
        if ctx.input(|i| i.viewport().close_requested()) {
            self.comms.send_expect(
                UiToOrchestratorCommand::EndGame,
                "Failed to send EndGame command",
            );
        }

        // ── Top control bar ─────────────────────────────────────────────
        ui::top_panel::show_top_panel(ctx, &mut self.ui_state, &self.comms);

        // ── Timer-based polling (not every frame!) ──────────────────────
        if self.timers.should_poll_planet_snapshots() {
            for planet in &self.galaxy_state.planets {
                if planet.active {
                    self.comms
                        .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet.id));
                }
            }
        }
        if self.timers.should_poll_explorer_snapshots() {
            for explorer_id in self.explorer_state.explorer_positions.keys() {
                self.comms
                    .send(UiToOrchestratorCommand::GetExplorerSnapshot(*explorer_id));
            }
        }
        if self.timers.should_poll_explorer_positions() {
            self.comms
                .send(UiToOrchestratorCommand::GetExplorersPosition);
        }

        // ── Drain expired refresh requests ─────────────────────────
        {
            let refresh_delay = std::time::Duration::from_millis(100);
            let mut i = 0;
            while i < self.animation_state.planets_to_refresh.len() {
                let (planet_id, queued_at) = self.animation_state.planets_to_refresh[i];
                if queued_at.elapsed() >= refresh_delay {
                    self.comms.send(
                        orchestrator::ui::UiToOrchestratorCommand::GetPlanetSnapshot(planet_id),
                    );
                    self.animation_state.planets_to_refresh.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        }

        // ── Central panel (canvas) ──────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            // 1. Process all pending orchestrator messages
            update_handler::handle_orchestrator_updates(
                &self.comms,
                &mut self.galaxy_state,
                &mut self.explorer_state,
                &mut self.animation_state,
                &mut self.ui_state,
            );

            // 2. Rebuild galaxy layout if the data changed
            let canvas_rect = ui.available_rect_before_wrap();
            let layout_center = ctx.content_rect().center();
            self.galaxy_state
                .rebuild_if_needed(canvas_rect, layout_center);

            // 3. Allocate painter
            let (response, painter) = ui.allocate_painter(canvas_rect.size(), egui::Sense::click());

            // 4. Draw space background
            ui::canvas::draw_background(&painter, canvas_rect);

            // 5. Handle spawn-planet menus
            ui::menus::handle_spawn_menus(
                ctx,
                &mut self.ui_state,
                &self.galaxy_state.planets,
                &self.comms,
            );

            // 6. Handle explorer move-to-planet selector
            ui::popups::show_move_selector(
                ctx,
                &mut self.ui_state,
                &self.explorer_state,
                &self.galaxy_state,
                &self.comms,
            );

            // 7. Draw edges
            self.galaxy_state.refresh_pos_cache();
            ui::canvas::draw_edges(
                &painter,
                &self.galaxy_state.edges,
                &self.galaxy_state.cached_pos_by_id,
            );

            // 8. Draw planets & explorers
            ui::canvas::draw_planets_and_explorers(
                ctx,
                &painter,
                &response,
                &self.galaxy_state,
                &self.explorer_state,
                &mut self.animation_state,
                &mut self.ui_state,
            );

            // 9. Show planet context menu
            let maybe_planet = self.ui_state.selected_planet;
            let maybe_pos = self.ui_state.context_menu_pos;
            if let (Some(planet_id), Some(menu_pos)) = (maybe_planet, maybe_pos) {
                // Only show planet menu if no explorer is selected
                if self.ui_state.selected_explorer.is_none() {
                    ui::menus::show_context_menu(
                        ctx,
                        menu_pos,
                        planet_id,
                        &mut self.ui_state,
                        &self.explorer_state,
                        &mut self.animation_state,
                        &self.comms,
                    );
                }
            }

            // 10. Show explorer context menu
            let maybe_explorer = self.ui_state.selected_explorer;
            let maybe_pos = self.ui_state.context_menu_pos;
            if let (Some(explorer_id), Some(menu_pos)) = (maybe_explorer, maybe_pos) {
                ui::menus::show_explorer_menu(
                    ctx,
                    menu_pos,
                    explorer_id,
                    &mut self.ui_state,
                    &self.comms,
                );
            }

            // 11. Show explorer-limit popup
            ui::popups::show_explorer_limit_popup(ctx, &mut self.ui_state);

            // 12. Show generate-resource popup
            ui::popups::show_generate_resource_popup(ctx, &mut self.ui_state, &self.comms);

            // 13. Show craft-resource popup
            ui::popups::show_craft_resource_popup(ctx, &mut self.ui_state, &self.comms);

            // 14. Draw asteroid animation
            ui::animations::draw_asteroid_animation(
                &painter,
                canvas_rect,
                &mut self.animation_state,
                &self.galaxy_state.planets,
            );

            // 15. Draw sunray animation
            ui::animations::draw_sunray_animation(
                &painter,
                canvas_rect,
                &mut self.animation_state,
                &self.galaxy_state.planets,
            );

            // 16. Draw instructions for the user
            ui::canvas::draw_help_text(&painter, canvas_rect);

            // don't block the UI thread here
        });

        // ── Schedule next repaint ───────────────────────────────────────
        if self.animation_state.has_active_animations() {
            // Fast repaints for visual effects (asteroid / sunray animations)
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            // Low-frequency repaint so polling timers keep firing and
            // orchestrator messages get drained even when the user is not
            // interacting.  This replaces the old approach that coupled
            // polling to the animation loop.
            let next_poll = self.timers.time_until_next_poll();
            // Add a small margin so we don't wake up just before the timer.
            ctx.request_repaint_after(next_poll + std::time::Duration::from_millis(50));
        }
    }
}
