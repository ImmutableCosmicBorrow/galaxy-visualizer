use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;
use eframe::egui;
use std::collections::HashSet;

use crate::models::SpawnStage;

pub struct UiState {
    pub pending_spawn_pos: Option<egui::Pos2>,
    pub spawn_stage: SpawnStage,
    pub selected_neighbors: HashSet<ID>,
    pub selected_planet: Option<ID>,
    pub context_menu_pos: Option<egui::Pos2>,
    pub selected_explorer: Option<ID>,
    pub pending_move_explorer: Option<ID>,
    pub pending_move_pos: Option<egui::Pos2>,
    pub explorer_limit_popup: Option<String>,
    pub game_over_popup: Option<String>,
    // Resource / crafting UI state
    pub pending_generate_explorer: Option<ID>,
    pub pending_craft_explorer: Option<ID>,
    pub resource_options: Option<Vec<BasicResourceType>>,
    pub combination_options: Option<Vec<ComplexResourceType>>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            pending_spawn_pos: None,
            spawn_stage: SpawnStage::None,
            selected_neighbors: HashSet::new(),
            selected_planet: None,
            context_menu_pos: None,
            selected_explorer: None,
            pending_move_explorer: None,
            pending_move_pos: None,
            explorer_limit_popup: None,
            game_over_popup: None,
            pending_generate_explorer: None,
            pending_craft_explorer: None,
            resource_options: None,
            combination_options: None,
        }
    }

    pub fn close_planet_menu(&mut self) {
        self.selected_planet = None;
        self.context_menu_pos = None;
    }

    pub fn close_explorer_menu(&mut self) {
        self.selected_explorer = None;
        self.context_menu_pos = None;
    }

    #[allow(dead_code)]
    pub fn cancel_spawn(&mut self) {
        self.pending_spawn_pos = None;
        self.spawn_stage = SpawnStage::None;
        self.selected_neighbors.clear();
    }
}
