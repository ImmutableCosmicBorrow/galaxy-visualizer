use common_game::components::planet::DummyPlanetState;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::logging::Channel;
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use orchestrator::ExplorerType;
use orchestrator::payload;
use orchestrator::planet::PlanetMap;
use orchestrator::ui::{OrchestratorToUiUpdate, UiToOrchestratorCommand};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::helpers::{build_planets_and_edges_from_galaxy, format_bag_content};
use crate::models::{Explorer, Planet, SpawnStage};

pub struct GalaxyApp {
    galaxy: Option<PlanetMap>,
    planets: Vec<Planet>,
    _explorers: Vec<Explorer>,
    edges: Vec<(ID, ID)>,
    explorer_positions: HashMap<ID, ID>, // explorer_id -> planet_id
    explorer_bags: HashMap<ID, common_explorer::ExplorerBagContent>,
    planet_states: HashMap<ID, DummyPlanetState>, // Track states separately
    cmd_sender: Sender<UiToOrchestratorCommand>,
    update_receiver: Receiver<OrchestratorToUiUpdate>,
    pending_spawn_pos: Option<egui::Pos2>,
    spawn_stage: SpawnStage,
    selected_neighbors: HashSet<ID>,
    selected_planet: Option<ID>,          // Currently selected planet
    context_menu_pos: Option<egui::Pos2>, // Position of context menu
    selected_explorer: Option<ID>,        // Currently selected explorer (for explorer context menu)
    pending_move_explorer: Option<ID>,    // Explorer awaiting move-to-planet selection
    pending_move_pos: Option<egui::Pos2>, // Position to show the neighbor selection menu
    explorer_limit_popup: Option<String>, // Popup message when explorer spawn limit reached
    // Resource/crafting UI state
    pending_generate_explorer: Option<ID>, // explorer awaiting generate-resource selection
    pending_craft_explorer: Option<ID>,    // explorer awaiting craft-resource selection
    resource_options: Option<Vec<BasicResourceType>>, // available basic resources
    combination_options: Option<Vec<ComplexResourceType>>, // available complex combinations

    sending_asteroid: Option<(ID, Instant)>,
    sending_sunray: Option<(ID, Instant)>,
    planets_to_refresh: Vec<(ID, Instant)>, // Planets needing state refresh after delay
    planet_displayed_charged: HashMap<ID, f32>, // animated displayed charged counter per planet

    planet_snapshot_timer: std::time::Instant,
    planet_snapshot_interval: std::time::Duration,
    explorer_snapshot_interval: std::time::Duration,
    explorer_snapshot_timer: std::time::Instant,
    explorer_position_timer: std::time::Instant,
    explorer_position_interval: std::time::Duration,
    
    // Performance optimization: cache to avoid rebuilding every frame
    galaxy_needs_rebuild: bool,
    cached_pos_by_id: HashMap<ID, egui::Pos2>,
}

impl GalaxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (mut orch, cmd_sender, update_receiver) = orchestrator::create_with_path(
            "galaxy/test_galaxy.txt",
            ExplorerType::Rob,
            Some(ExplorerType::Nico),
            None,
            2000,
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
            galaxy: None,
            planets: Vec::new(),
            _explorers: Vec::new(),
            edges: Vec::new(),
            explorer_positions: HashMap::new(),
            explorer_bags: HashMap::new(),
            planet_states: HashMap::new(),
            cmd_sender,
            update_receiver,
            pending_spawn_pos: None,
            spawn_stage: SpawnStage::None,
            selected_neighbors: HashSet::new(),
            selected_planet: None,
            context_menu_pos: None,
            selected_explorer: None,
            pending_move_explorer: None,
            pending_move_pos: None,
            explorer_limit_popup: None,
            pending_generate_explorer: None,
            pending_craft_explorer: None,
            resource_options: None,
            combination_options: None,
            sending_asteroid: None,
            sending_sunray: None,
            planets_to_refresh: Vec::new(),
            planet_displayed_charged: HashMap::new(),
            planet_snapshot_timer: std::time::Instant::now(),
            planet_snapshot_interval: std::time::Duration::from_millis(200), 
            explorer_snapshot_interval: std::time::Duration::from_millis(200), 
            explorer_snapshot_timer: std::time::Instant::now(),
            explorer_position_timer: std::time::Instant::now(),
            explorer_position_interval: std::time::Duration::from_millis(200), 
            galaxy_needs_rebuild: true,
            cached_pos_by_id: HashMap::new(),
        }
    }

    fn show_planet_type_menu(&mut self, ctx: &egui::Context, pos: egui::Pos2) {
        let mut chosen_id: Option<ID> = None;

        egui::Area::new(egui::Id::new("planet_type_menu"))
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Select Planet Type:");
                    if ui.button("Rusty Crabs").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_rusty_crab_id());
                    } else if ui.button("Rustrelli").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_rustrelli_id());
                    } else if ui.button("Orbitron").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_orbitron_id());
                    } else if ui.button("Houston we have a borrow").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_houston_id());
                    } else if ui.button("Trip").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_trip_id());
                    } else if ui.button("Luna4").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_luna4_id());
                    } else if ui.button("Enterprise").clicked() {
                        chosen_id = Some(orchestrator::get_id_manager().get_next_enterprise_id());
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending_spawn_pos = None;
                        self.spawn_stage = SpawnStage::None;
                    }
                });
            });

        if let Some(new_id) = chosen_id {
            self.spawn_stage = SpawnStage::SelectingNeighbors(new_id);
            self.selected_neighbors.clear();
        }
    }

    fn show_neighbor_selection_menu(
        &mut self,
        ctx: &egui::Context,
        pos: egui::Pos2,
        planet_id: ID,
    ) {
        let mut confirm_pressed = false;
        let mut cancel_pressed = false;

        egui::Area::new(egui::Id::new("neighbor_selection_menu"))
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Select Neighbors (or none):");

                    for planet in &self.planets {
                        let mut is_selected = self.selected_neighbors.contains(&planet.id);
                        ui.checkbox(&mut is_selected, format!("Planet {}", planet.id));
                        if is_selected {
                            self.selected_neighbors.insert(planet.id);
                        } else {
                            self.selected_neighbors.remove(&planet.id);
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("✓ Confirm").clicked() {
                            confirm_pressed = true;
                        }
                        if ui.button("✗ Cancel").clicked() {
                            cancel_pressed = true;
                        }
                    });
                });
            });

        if confirm_pressed {
            let neighbors: Vec<ID> = self.selected_neighbors.iter().copied().collect();
            // Don't add to self.planets manually - let the galaxy rebuild handle positioning
            // This ensures all planets are arranged in the circle layout
            
            self.cmd_sender
                .send(UiToOrchestratorCommand::AddPlanet(planet_id, neighbors))
                .expect("Failed to send AddPlanet command");
            
            // Request galaxy update to get proper circular positioning
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetGalaxy)
                .expect("Failed to send GetGalaxy command");

            self.pending_spawn_pos = None;
            self.spawn_stage = SpawnStage::None;
            self.selected_neighbors.clear();
        }

        if cancel_pressed {
            self.pending_spawn_pos = None;
            self.spawn_stage = SpawnStage::None;
            self.selected_neighbors.clear();
        }
    }

    fn show_context_menu(&mut self, ctx: &egui::Context, pos: egui::Pos2, planet_id: ID) {
        let mut manual_asteroid = false;
        let mut manual_sunray = false;
        let mut start_ai = false;
        let mut stop_ai = false;
        let mut reset_ai = false;
        let mut spawn_nico_explorer = false; //TODO: make it choose expl type with the following window
        let mut spawn_rob_explorer = false;
        let mut spawn_jaco_explorer = false;
        let mut close_menu = false;
        //TODO: does it need internal state?

        egui::Area::new(egui::Id::new("context_menu"))
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(format!("Planet {planet_id}"));
                    ui.separator();

                    if ui.button("👤 Spawn Nico Explorer").clicked() {
                        spawn_nico_explorer = true;
                    }
                    if ui.button("👤 Spawn Rob Explorer").clicked() {
                        spawn_rob_explorer = true;
                    }
                    if ui.button("👤 Spawn Jaco Explorer").clicked() {
                        spawn_jaco_explorer = true;
                    }
                    if ui.button("Send Asteroid").clicked() {
                        manual_asteroid = true;
                    }
                    if ui.button("Send Sunray").clicked() {
                        manual_sunray = true;
                    }
                    if ui.button("Start Planet AI").clicked() {
                        start_ai = true;
                    }
                    if ui.button("Stop Planet AI").clicked() {
                        stop_ai = true;
                    }
                    if ui.button("Reset Planet AI").clicked() {
                        reset_ai = true;
                    }
                    if ui.button("✗ Close").clicked() {
                        close_menu = true;
                    }
                });
            });

        // Handling selections outside the closure to avoid borrow issues
        if spawn_nico_explorer || spawn_rob_explorer || spawn_jaco_explorer {
            let expl_type;
            if spawn_nico_explorer {
                expl_type = ExplorerType::Nico;
            } else if spawn_rob_explorer {
                expl_type = ExplorerType::Rob;
            } else {
                expl_type = ExplorerType::Jaco;
            }

            // Use the authoritative explorer positions map for counting explorers
            let explorer_count = self.explorer_positions.len();
            if explorer_count >= 2 {
                let msg = format!(
                    "❗ Explorer limit reached ({} explorers), cannot spawn more explorers.",
                    explorer_count
                );
                orchestrator::logging_utils::log_internal(
                    Channel::Warning,
                    payload!(
                        message : msg.clone(),
                        explorer_count : explorer_count,
                    ),
                );
                self.explorer_limit_popup = Some(msg);
            } else {
                self.cmd_sender
                    .send(UiToOrchestratorCommand::AddExplorer(expl_type, planet_id))
                    .ok();
                // Immediately request updated explorer positions and planet state
                self.cmd_sender
                    .send(UiToOrchestratorCommand::GetExplorersPosition)
                    .ok();
                self.cmd_sender
                    .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                    .ok();
            }
            close_menu = true;
        }

        if manual_asteroid {
            self.cmd_sender
                .send(UiToOrchestratorCommand::SendManualAsteroid(planet_id))
                .ok();
            self.sending_asteroid = Some((planet_id, Instant::now()));
            // Schedule refresh after 100ms to let orchestrator process the asteroid
            self.planets_to_refresh.push((planet_id, Instant::now()));
            // Request galaxy update to catch planet death
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetGalaxy)
                .ok();
            // Request planet snapshot to see damage/death
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                .ok();
            close_menu = true;
        }

        if manual_sunray {
            orchestrator::logging_utils::log_internal(
                Channel::Info,
                payload!(
                    action : "send_manual_sunray",
                    planet_id : planet_id,
                ),
            );
            self.cmd_sender
                .send(UiToOrchestratorCommand::SendManualSunray(planet_id))
                .ok();
            self.sending_sunray = Some((planet_id, Instant::now()));
            // Schedule refresh after 100ms to let orchestrator process the sunray
            self.planets_to_refresh.push((planet_id, Instant::now()));

            // Request planet snapshot to see sunray effect
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                .ok();

            close_menu = true;
        }

        if start_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::StartPlanetAI(planet_id))
                .ok();
            // Request immediate update to see rocket status changes
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                .ok();
            close_menu = true;
        }

        if stop_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::StopPlanetAI(planet_id))
                .ok();
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                .ok();
            close_menu = true;
        }

        if reset_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::ResetPlanetAI(planet_id))
                .ok();
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                .ok();
            close_menu = true;
        }

        if close_menu {
            self.selected_planet = None;
            self.context_menu_pos = None;
        }
    }

    fn show_explorer_menu(&mut self, ctx: &egui::Context, pos: egui::Pos2, explorer_id: ID) {
        let mut stop_expl_ai = false;
        let mut reset_expl_ai = false;
        let mut start_expl_ai = false;
        let mut move_to_planet = false;
        let mut generate_resource = false;
        let mut craft_resource = false;
        let mut close = false;

        egui::Area::new(egui::Id::new("explorer_context_menu"))
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(format!("Explorer {explorer_id}"));
                    ui.separator();

                    if ui.button("Move To Planet").clicked() {
                        // open neighbor-only selector after closing this menu
                        move_to_planet = true;
                    }
                    if ui.button("Generate Resource").clicked() {
                        generate_resource = true;
                    }
                    if ui.button("Craft Resource").clicked() {
                        craft_resource = true;
                    }
                    if ui.button("Start Explorer AI").clicked() {
                        start_expl_ai = true;
                    }
                    if ui.button("Stop Explorer AI").clicked() {
                        stop_expl_ai = true;
                    }
                    if ui.button("Reset Explorer AI").clicked() {
                        reset_expl_ai = true;
                    }

                    if ui.button("✗ Close").clicked() {
                        close = true;
                    }
                });
            });
        if start_expl_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::StartExplorerAI(explorer_id))
                .ok();
            // Request updates to track explorer movement
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetExplorersPosition)
                .ok();
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetExplorerSnapshot(explorer_id))
                .ok();
            close = true;
        }

        if reset_expl_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::ResetExplorerAI(explorer_id))
                .ok();
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetExplorersPosition)
                .ok();
            close = true;
        }

        if stop_expl_ai {
            self.cmd_sender
                .send(UiToOrchestratorCommand::StopExplorerAI(explorer_id))
                .ok();
            self.cmd_sender
                .send(UiToOrchestratorCommand::GetExplorersPosition)
                .ok();
            close = true;
        }

        if move_to_planet {
            // prepare neighbor-only selector: record explorer id and position
            self.pending_move_explorer = Some(explorer_id);
            self.pending_move_pos = Some(pos);
            close = true;
        }

        if generate_resource {
            // ask orchestrator for supported resources for the planet where this explorer is
            self.pending_generate_explorer = Some(explorer_id);
            // clear previous options
            self.resource_options = None;
            orchestrator::logging_utils::log_internal(
                Channel::Info,
                payload!(
                    action : "request_supported_resources",
                    requester_explorer_id : explorer_id,
                ),
            );
            let _ = self
                .cmd_sender
                .send(UiToOrchestratorCommand::SupportedResources(explorer_id));

            close = true;
        }

        if craft_resource {
            // ask orchestrator for supported combinations for this explorer, then show selection
            self.pending_craft_explorer = Some(explorer_id);
            self.combination_options = None;
            orchestrator::logging_utils::log_internal(
                Channel::Info,
                payload!(
                    action : "request_supported_combinations",
                    requester_explorer_id : explorer_id,
                ),
            );
            let _ = self
                .cmd_sender
                .send(UiToOrchestratorCommand::SupportedCombinations(explorer_id));
            close = true;
        }

        if close {
            self.selected_explorer = None;
            self.context_menu_pos = None;
        }
    }
}

#[allow(clippy::too_many_lines)]
impl eframe::App for GalaxyApp {
    #[allow(clippy::cast_possible_truncation)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle window close (X button)
        if ctx.input(|i| i.viewport().close_requested()) {
            self.cmd_sender
                .send(UiToOrchestratorCommand::EndGame)
                .expect("Failed to send EndGame command");
        }
        egui::TopBottomPanel::top("top_controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left side buttons
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.button("Switch Game Mode").clicked() {
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::SwitchGameMode)
                            .expect("Failed to send SwitchGameMode command");
                    }
                    if ui.button("End Game").clicked() {
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::EndGame)
                            .expect("Failed to send EndGame command");
                    }
                    if ui.button("Pause Game").clicked() {
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::PauseGame)
                            .expect("Failed to send PauseGame command");
                    }

                    if ui.button("Resume Game").clicked() {
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::ResumeGame)
                            .expect("Failed to send ResumeGame command");
                    }
                });

                // Right side button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Create Planet").clicked() {
                        self.pending_spawn_pos =
                            Some(ui.max_rect().right_top() + egui::vec2(-10.0, 30.0));
                        self.spawn_stage = SpawnStage::SelectingType;
                        self.selected_neighbors.clear();
                    }
                });
            });
        });

        // Request planet snapshots on timer (not every frame!)
        if self.planet_snapshot_timer.elapsed() >= self.planet_snapshot_interval {
            for planet in &self.planets {
                let _ = self
                    .cmd_sender
                    .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet.id));
            }
            self.planet_snapshot_timer = std::time::Instant::now();
        }

        // Request explorer snapshots on timer (not every frame!)
        if self.explorer_snapshot_timer.elapsed() >= self.explorer_snapshot_interval {
            for (explorer_id, _) in &self.explorer_positions {
                let _ = self
                    .cmd_sender
                    .send(UiToOrchestratorCommand::GetExplorerSnapshot(*explorer_id));
            }
            self.explorer_snapshot_timer = std::time::Instant::now();
        }

        // Continuously monitor explorer positions to catch movement
        if self.explorer_position_timer.elapsed() >= self.explorer_position_interval {
            let _ = self
                .cmd_sender
                .send(UiToOrchestratorCommand::GetExplorersPosition);
            self.explorer_position_timer = std::time::Instant::now();
        }

        // Only request explorer positions periodically, not every frame
        // Use events and explicit requests instead

        egui::CentralPanel::default().show(ctx, |ui| {
            // Check for updates from orchestrator thread
            while let Ok(cmd) = self.update_receiver.try_recv() {
                match cmd {
                    OrchestratorToUiUpdate::Galaxy(galaxy) => {
                        self.galaxy = Some(galaxy);
                        self.galaxy_needs_rebuild = true;
                    }
                    OrchestratorToUiUpdate::DeadPlanet(id) => {
                        self.planets.iter_mut().for_each(|planet| {
                            if planet.id == id {
                                planet.active = false;
                            }
                        });
                        // Request galaxy update to ensure we have latest state
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::GetGalaxy)
                            .ok();
                        // Remove any explorers on this planet from the positions map
                        let dead_explorers: Vec<ID> = self
                            .explorer_positions
                            .iter()
                            .filter(|(_, planet_id)| **planet_id == id)
                            .map(|(explorer_id, _)| *explorer_id)
                            .collect();

                        for explorer_id in dead_explorers {
                            self.explorer_positions.remove(&explorer_id);
                            self.explorer_bags.remove(&explorer_id);
                            // Clean up any pending operations for dead explorers
                            if self.pending_generate_explorer == Some(explorer_id) {
                                self.pending_generate_explorer = None;
                                self.resource_options = None;
                            }
                            if self.pending_craft_explorer == Some(explorer_id) {
                                self.pending_craft_explorer = None;
                                self.combination_options = None;
                            }
                        }

                        // Request fresh explorer positions from orchestrator
                        let _ = self
                            .cmd_sender
                            .send(UiToOrchestratorCommand::GetExplorersPosition);
                    }
                    OrchestratorToUiUpdate::ExplorersPosition(positions) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Debug,
                            payload!(
                                action : "received_explorers_position",
                            ),
                        );
                        let guard = positions
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        //println!("← Received ExplorersPosition: {:?}", &*guard);
                        self.explorer_positions.clone_from(&*guard);
                        //println!("   Updated self.explorer_positions: {:?}", self.explorer_positions);
                    }
                    OrchestratorToUiUpdate::PlanetSnapshot(id, snapshot) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Debug,
                            payload!(
                                action : "received_planet_snapshot",
                                planet_id : id,
                            ),
                        );
                        // store actual snapshot
                        self.planet_states.insert(id, snapshot.clone());
                        // ensure we have a displayed counter initialized so it can animate
                        self.planet_displayed_charged
                            .entry(id)
                            .or_insert(snapshot.charged_cells_count as f32);
                    }
                    OrchestratorToUiUpdate::ExplorerSnapshot(id, bag) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Debug,
                            payload!(
                                action : "received_explorer_snapshot",
                                explorer_id : id,
                                bag: format!("{:?}", bag)
                            ),
                        );
                        self.explorer_bags.insert(id, bag);
                    }

                    //draw supported combinations/resources, spawned when someone wants to craft/combine
                    OrchestratorToUiUpdate::SupportedCombinations(explorer_id, combinations) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Info,
                            payload!(
                                action : "received_supported_combinations",
                                explorer_id : explorer_id,
                                supported_combo: format!("{:?}", combinations)
                            ),
                        );
                        let vec: Vec<ComplexResourceType> = combinations.into_iter().collect();
                        if self.pending_craft_explorer == Some(explorer_id) {
                            self.combination_options = Some(vec);
                        }
                    }
                    OrchestratorToUiUpdate::SupportedResources(explorer_id, resources) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Info,
                            payload!(
                                action : "received_supported_resources",
                                explorer_id : explorer_id,
                                supported_resources: format!("{:?}", resources)
                            ),
                        );
                        let vec: Vec<BasicResourceType> = resources.into_iter().collect();
                        // cache by planet id (look up planet from explorer position)
                        if self.pending_generate_explorer == Some(explorer_id) {
                            self.resource_options = Some(vec);
                        }
                    }

                    //just draw sunray/asteroid
                    OrchestratorToUiUpdate::SendAutoSunray(planet_id) => {
                        orchestrator::logging_utils::log_internal(
                            Channel::Debug,
                            payload!(
                                action : "auto sunray received: drawing it",
                                planet_id : planet_id,
                            ),
                        );
                        self.sending_sunray = Some((planet_id, Instant::now()));
                        // Schedule refresh after a short delay to let orchestrator process the sunray
                        self.planets_to_refresh.push((planet_id, Instant::now()));
                        // Request immediate snapshot to catch rocket status
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                            .ok();
                        // Clamp displayed charged counter to the maximum number of energy cells
                        if let Some(state) = self.planet_states.get(&planet_id) {
                            let max_charged = state.energy_cells.len() as f32;
                            self.planet_displayed_charged
                                .entry(planet_id)
                                .and_modify(|v| {
                                    if *v > max_charged {
                                        *v = max_charged;
                                    }
                                })
                                .or_insert(max_charged);
                        }
                    }
                    OrchestratorToUiUpdate::SendAutoAsteroid(planet_id) => {
                        self.sending_asteroid = Some((planet_id, Instant::now()));
                        // Schedule refresh after 100ms to let orchestrator process the asteroid
                        self.planets_to_refresh.push((planet_id, Instant::now()));
                        // Request galaxy update to catch planet death
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::GetGalaxy)
                            .ok();
                        // Request immediate snapshot to catch planet death or damage
                        self.cmd_sender
                            .send(UiToOrchestratorCommand::GetPlanetSnapshot(planet_id))
                            .ok();
                        orchestrator::logging_utils::log_internal(
                            Channel::Debug,
                            payload!(
                                action : "auto asteroid received: drawing it",
                                planet_id : planet_id,
                            ),
                        );
                    }
                }
            }

            // reserve the whole screen for painting
            let canvas_rect = ui.available_rect_before_wrap();
            
            // Only rebuild galaxy layout when it changes, not every frame!
            if self.galaxy_needs_rebuild {
                if let Some(galaxy) = &self.galaxy {
                    let center = canvas_rect.center();
                    let radius = (canvas_rect.width().min(canvas_rect.height()) * 0.35).max(50.0);
                    let (planets, edges) = build_planets_and_edges_from_galaxy(galaxy, center, radius);
                    self.planets = planets;
                    self.edges = edges;
                    self.galaxy_needs_rebuild = false;
                }
            }
            let (response, painter) = ui.allocate_painter(canvas_rect.size(), egui::Sense::click());

            // draw space background drak blue
            painter.rect_filled(
                canvas_rect,
                0.0, // Corner rounding
                egui::Color32::from_rgb(10, 10, 25),
            );

            // Handle spawn menu UI
            if let Some(pos) = self.pending_spawn_pos {
                match self.spawn_stage {
                    SpawnStage::SelectingType => {
                        self.show_planet_type_menu(ctx, pos);
                    }
                    SpawnStage::SelectingNeighbors(planet_id) => {
                        self.show_neighbor_selection_menu(ctx, pos, planet_id);
                    }
                    SpawnStage::None => {}
                }
            }

            // If an explorer requested a move-to-planet, show neighbor-only selector
            if let Some(explorer_id) = self.pending_move_explorer
                && let Some(pos) = self.pending_move_pos
            {
                // determine current planet for this explorer
                if let Some(current_planet) = self.explorer_positions.get(&explorer_id).copied() {
                    // collect neighbors from galaxy snapshot
                    let mut neighbors: Vec<ID> = Vec::new();
                    if let Some(galaxy) = &self.galaxy {
                        let guard = galaxy
                            .read()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if let Some(node) = guard.get(&current_planet) {
                            neighbors = node.neighbors_snapshot();
                        }
                    }

                    egui::Area::new(egui::Id::new("explorer_move_selector"))
                        .fixed_pos(pos)
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                ui.label(format!(
                                    "Move Explorer {explorer_id} from Planet {current_planet} to:"
                                ));
                                ui.separator();

                                if neighbors.is_empty() {
                                    ui.label("No neighbors available");
                                } else {
                                    for nid in &neighbors {
                                        if ui.button(format!("Planet {nid}")).clicked() {
                                            // log and send move command: Explorer ID, from, to
                                            orchestrator::logging_utils::log_internal(
                                                Channel::Info,
                                                payload!(
                                                    action : "manual_move_explorer",
                                                    explorer_id : explorer_id,
                                                    from_planet : current_planet,
                                                    to_planet : *nid,
                                                ),
                                            );
                                            let _ = self.cmd_sender.send(
                                                UiToOrchestratorCommand::ManualMoveExplorer(
                                                    explorer_id,
                                                    current_planet,
                                                    *nid,
                                                ),
                                            );
                                            // request refreshed positions
                                            let _ = self.cmd_sender.send(
                                                UiToOrchestratorCommand::GetExplorersPosition,
                                            );
                                            // clear pending selector
                                            self.pending_move_explorer = None;
                                            self.pending_move_pos = None;
                                        }
                                    }
                                }

                                ui.separator();
                                if ui.button("✗ Cancel").clicked() {
                                    self.pending_move_explorer = None;
                                    self.pending_move_pos = None;
                                }
                            });
                        });
                } else {
                    // Explorer has no known planet: just clear selector
                    self.pending_move_explorer = None;
                    self.pending_move_pos = None;
                }
            }

            // draw edges (from neighbors_snapshot)
            // Update cached positions only if galaxy was rebuilt
            if self.cached_pos_by_id.is_empty() || self.cached_pos_by_id.len() != self.planets.len() {
                self.cached_pos_by_id.clear();
                for planet in &self.planets {
                    self.cached_pos_by_id.insert(planet.id, planet.pos);
                }
            }
            
            for (a, b) in &self.edges {
                if let (Some(pa), Some(pb)) = (self.cached_pos_by_id.get(a), self.cached_pos_by_id.get(b)) {
                    painter.line_segment(
                        [*pa, *pb],
                        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)),
                    );
                }
            }

            // draw planets
            for planet in &self.planets {
                // size
                let radius = 20.0;

                // Check for right-click on this planet
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let distance = pointer_pos.distance(planet.pos);
                    if distance <= radius && response.clicked_by(egui::PointerButton::Secondary) {
                        self.selected_planet = Some(planet.id);
                        self.context_menu_pos = Some(pointer_pos);
                    }
                }

                // Draw colored shadow if planet is selected
                if self.selected_planet == Some(planet.id) {
                    painter.circle_filled(
                        planet.pos,
                        radius + 5.0,
                        egui::Color32::from_rgba_unmultiplied(200, 150, 255, 100), // purple glow
                    );
                }

                // TODO: use Kenney asset
                /*
                painter.image(
                    my_planet_texture_id,
                    egui::Rect::from_center_size(planet.pos, egui::Vec2::splat(50.0)),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    planet.color // Tinting!
                );
                */

                if planet.active {
                    painter.circle_filled(
                        planet.pos,
                        radius,
                        egui::Color32::from_rgb(100, 200, 255),
                    );
                } else {
                    painter.circle_filled(
                        planet.pos,
                        radius,
                        egui::Color32::from_rgba_unmultiplied(50, 50, 50, 200),
                    );
                }

                // draw explorers on this planet as small colored dots
                let explorers_on_planet: Vec<ID> = self
                    .explorer_positions
                    .iter()
                    .filter(|(_, planet_id)| **planet_id == planet.id)
                    .map(|(explorer_id, _)| *explorer_id)
                    .collect();

                #[allow(clippy::cast_precision_loss)]
                for (i, explorer_id) in explorers_on_planet.iter().enumerate() {
                    let angle = (i as f32 / explorers_on_planet.len().max(1) as f32)
                        * std::f32::consts::TAU;
                    let explorer_pos = planet.pos
                        + egui::Vec2::new(
                            (radius + 12.0) * angle.cos(),
                            (radius + 12.0) * angle.sin(),
                        );
                    // Explorer visual parameters
                    let explorer_radius = 8.0; // made larger per request

                    // Check for right-click on explorer
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let distance = pointer_pos.distance(explorer_pos);
                        if distance <= explorer_radius
                            && response.clicked_by(egui::PointerButton::Secondary)
                        {
                            self.selected_explorer = Some(*explorer_id);
                            self.selected_planet = None;
                            self.context_menu_pos = Some(pointer_pos);
                        }
                    }

                    // Draw explorer as a larger bright yellow dot
                    painter.circle_filled(explorer_pos, explorer_radius, egui::Color32::YELLOW);

                    // Draw explorer label with name, ID, and bag content
                    let explorer_name =
                        orchestrator::id::IdManager::explorer_name_from_id(*explorer_id);
                    let bag_line = match self.explorer_bags.get(explorer_id) {
                        Some(bag) => format_bag_content(bag),
                        None => "bag: loading".to_string(),
                    };
                    let label_text = format!("{explorer_name} {explorer_id}\n{bag_line}");
                    painter.text(
                        explorer_pos
                            + egui::Vec2::new(explorer_radius + 6.0, -explorer_radius - 6.0),
                        egui::Align2::LEFT_TOP,
                        label_text,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                }

                // draw name label centered below planet
                painter.text(
                    planet.pos + egui::Vec2::new(0.0, radius + 10.0),
                    egui::Align2::CENTER_TOP,
                    &planet.name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );

                // Draw planet state if available
                if let Some(state) = self.planet_states.get(&planet.id) {
                    let state_pos = planet.pos + egui::Vec2::new(0.0, radius + 28.0);

                    // Animate displayed charged counter towards actual value
                    let dt = ctx.input(|i| i.stable_dt);
                    let target_charged = state.charged_cells_count as f32;
                    let displayed = self
                        .planet_displayed_charged
                        .entry(planet.id)
                        .or_insert(target_charged);
                    let speed_per_sec = 20.0; // units per second
                    let step = speed_per_sec * dt;
                    
                    // Animate in BOTH directions (up and down)
                    if (*displayed - target_charged).abs() > 0.01 {
                        if *displayed < target_charged {
                            *displayed = (*displayed + step).min(target_charged);
                        } else {
                            *displayed = (*displayed - step).max(target_charged);
                        }
                    }

                    // use the animated value when building text
                    let displayed_charged_usize = (*displayed).round() as usize;

                    // Build state text with emojis (animated value)
                    let mut state_text = String::new();
                    if state.has_rocket {
                        state_text.push_str("🚀 ");
                    }
                    state_text.push_str(&format!(
                        "⚡{}/{} ",
                        displayed_charged_usize,
                        state.energy_cells.len()
                    ));

                    // Draw state text - use simpler rendering without extra layout
                    painter.text(
                        state_pos,
                        egui::Align2::CENTER_TOP,
                        state_text,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                }
            }

            // Show context menu if a planet is selected
            if let Some(planet_id) = self.selected_planet
                && let Some(menu_pos) = self.context_menu_pos
            {
                self.show_context_menu(ctx, menu_pos, planet_id);
            }

            // Show explorer context menu if an explorer is selected
            if let Some(explorer_id) = self.selected_explorer
                && let Some(menu_pos) = self.context_menu_pos
            {
                self.show_explorer_menu(ctx, menu_pos, explorer_id);
            }

            // Show explorer-limit popup if set (clone message to avoid borrowing `self` inside closure)
            if let Some(msg) = self.explorer_limit_popup.clone() {
                egui::Window::new("Notice")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(ctx, |ui| {
                        ui.label(msg);
                        if ui.button("OK").clicked() {
                            self.explorer_limit_popup = None;
                        }
                    });
            }

            // If we have resource options for generation, show them
            if let Some(expl_id) = self.pending_generate_explorer {
                egui::Area::new(egui::Id::new("generate_resource_menu"))
                    .fixed_pos(egui::Pos2::new(100.0, 100.0))
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.label(format!("Generate resource for Explorer {expl_id}:"));
                            ui.separator();

                            if let Some(res_options) = self.resource_options.as_ref() {
                                if res_options.is_empty() {
                                    ui.label("No resources can be generated here.");
                                } else {
                                    let mut chosen_resource: Option<BasicResourceType> = None;
                                    for opt in res_options {
                                        let label = format!("{opt:?}");
                                        if ui.button(label).clicked() {
                                            chosen_resource = Some(*opt);
                                        }
                                    }
                                    if let Some(res) = chosen_resource {
                                        orchestrator::logging_utils::log_internal(
                                            Channel::Info,
                                            payload!(
                                                action : "explorer_generate_resource",
                                                explorer_id : expl_id,
                                                resource : format!("{res:?}"),
                                            ),
                                        );
                                        let _ = self.cmd_sender.send(
                                            UiToOrchestratorCommand::ExplorerGenerateResource(
                                                expl_id, res,
                                            ),
                                        );
                                        let _ = self.cmd_sender.send(
                                            UiToOrchestratorCommand::GetExplorerSnapshot(expl_id),
                                        );
                                        let _ = self
                                            .cmd_sender
                                            .send(UiToOrchestratorCommand::GetExplorersPosition);
                                        self.pending_generate_explorer = None;
                                        self.resource_options = None;
                                    }
                                }
                            } else {
                                ui.label("Loading...");
                            }

                            ui.separator();
                            if ui.button("✗ Cancel").clicked() {
                                self.pending_generate_explorer = None;
                                self.resource_options = None;
                            }
                        });
                    });
            }

            // If we have combination options for crafting, show them
            if let Some(expl_id) = self.pending_craft_explorer {
                egui::Area::new(egui::Id::new("craft_resource_menu"))
                    .fixed_pos(egui::Pos2::new(100.0, 100.0))
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.label(format!("Craft resource for Explorer {expl_id}:"));
                            ui.separator();

                            if let Some(comb_options) = self.combination_options.as_ref() {
                                if comb_options.is_empty() {
                                    ui.label("No resources can be generated here.");
                                } else {
                                    let mut chosen_resource: Option<ComplexResourceType> = None;
                                    for opt in comb_options {
                                        let label = format!("{opt:?}");
                                        if ui.button(label).clicked() {
                                            chosen_resource = Some(*opt);
                                        }
                                    }
                                    if let Some(res) = chosen_resource {
                                        orchestrator::logging_utils::log_internal(
                                            Channel::Info,
                                            payload!(
                                                action : "explorer_generate_resource",
                                                explorer_id : expl_id,
                                                resource : format!("{res:?}"),
                                            ),
                                        );
                                        let _ = self.cmd_sender.send(
                                            UiToOrchestratorCommand::ExplorerCombineResource(
                                                expl_id, res,
                                            ),
                                        );
                                        let _ = self.cmd_sender.send(
                                            UiToOrchestratorCommand::GetExplorerSnapshot(expl_id),
                                        );
                                        let _ = self
                                            .cmd_sender
                                            .send(UiToOrchestratorCommand::GetExplorersPosition);
                                        self.pending_craft_explorer = None;
                                        self.combination_options = None;
                                    }
                                }
                            } else {
                                ui.label("Loading...");
                            }

                            ui.separator();
                            if ui.button("✗ Cancel").clicked() {
                                self.pending_generate_explorer = None;
                                self.resource_options = None;
                            }
                        });
                    });
            }

            if let Some((planet_id, start_time)) = self.sending_asteroid {
                // Make asteroid effect immediate: draw full trail and a short flash
                let display_duration = 0.25_f32; // seconds to keep the flash
                let elapsed = start_time.elapsed().as_secs_f32();
                if let Some(planet) = self.planets.iter().find(|p| p.id == planet_id) {
                    let origin = canvas_rect.center();

                    // Full trail instantly
                    painter.line_segment(
                        [origin, planet.pos],
                        egui::Stroke::new(
                            3.0,
                            egui::Color32::from_rgba_unmultiplied(255, 80, 80, 220),
                        ),
                    );

                    // Impact dot
                    painter.circle_filled(planet.pos, 6.0, egui::Color32::RED);

                    // Expanding ring to emphasize impact
                    let ring_progress = (elapsed / display_duration).min(1.0);
                    let ring_radius = 10.0 + 30.0 * ring_progress;
                    let ring_alpha = ((1.0 - ring_progress) * 200.0).max(0.0) as u8;
                    painter.circle_stroke(
                        planet.pos,
                        ring_radius,
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 80, 80, ring_alpha),
                        ),
                    );

                    if elapsed >= display_duration {
                        self.sending_asteroid = None;
                    } else {
                        self.sending_asteroid = Some((planet_id, start_time));
                    }
                } else {
                    self.sending_asteroid = None;
                }
            }

            if let Some((planet_id, start_time)) = self.sending_sunray {
                // Make sunray feel immediate: full beam + quick flash
                let display_duration = 0.15_f32;
                let elapsed = start_time.elapsed().as_secs_f32();
                if let Some(planet) = self.planets.iter().find(|p| p.id == planet_id) {
                    let origin = canvas_rect.center();

                    // Full beam immediately
                    painter.line_segment(
                        [origin, planet.pos],
                        egui::Stroke::new(
                            3.0,
                            egui::Color32::from_rgba_unmultiplied(255, 230, 120, 220),
                        ),
                    );

                    // Burst at target
                    painter.circle_filled(planet.pos, 6.0, egui::Color32::YELLOW);

                    // Subtle expanding glow
                    let ring_progress = (elapsed / display_duration).min(1.0);
                    let ring_radius = 12.0 + 28.0 * ring_progress;
                    let ring_alpha = ((1.0 - ring_progress) * 180.0).max(0.0) as u8;
                    painter.circle_stroke(
                        planet.pos,
                        ring_radius,
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(255, 230, 120, ring_alpha),
                        ),
                    );

                    if elapsed >= display_duration {
                        self.sending_sunray = None;
                    } else {
                        self.sending_sunray = Some((planet_id, start_time));
                    }
                } else {
                    self.sending_sunray = None;
                }
            }

            // instructions for the user to draw new planet
            painter.text(
                canvas_rect.min + egui::Vec2::new(20.0, 20.0),
                egui::Align2::LEFT_TOP,
                "Right-click a planet for options. Use the top-right button to create a planet.",
                egui::FontId::monospace(14.0),
                egui::Color32::YELLOW,
            );

            // don't block the UI thread here
        });
        
        // Only request repaints when we have active animations
        if self.sending_asteroid.is_some() 
            || self.sending_sunray.is_some() 
            || !self.planets_to_refresh.is_empty()
            || self.planet_displayed_charged.iter().any(|(id, displayed)| {
                if let Some(state) = self.planet_states.get(id) {
                    (*displayed - state.charged_cells_count as f32).abs() > 0.01
                } else {
                    false
                }
            })
        {
            ctx.request_repaint();
        }
    }
}
