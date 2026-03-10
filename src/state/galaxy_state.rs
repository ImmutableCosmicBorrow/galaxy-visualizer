use common_game::components::planet::DummyPlanetState;
use common_game::utils::ID;
use eframe::egui;
use orchestrator::planet::PlanetMap;
use std::collections::HashMap;

use crate::helpers::build_planets_and_edges_from_galaxy;
use crate::models::Planet;

pub struct GalaxyState {
    pub galaxy: Option<PlanetMap>,
    pub planets: Vec<Planet>,
    pub edges: Vec<(ID, ID)>,
    pub planet_states: HashMap<ID, DummyPlanetState>,
    pub galaxy_needs_rebuild: bool,
    pub cached_pos_by_id: HashMap<ID, egui::Pos2>,
}

impl GalaxyState {
    pub fn new() -> Self {
        Self {
            galaxy: None,
            planets: Vec::new(),
            edges: Vec::new(),
            planet_states: HashMap::new(),
            galaxy_needs_rebuild: true,
            cached_pos_by_id: HashMap::new(),
        }
    }

    /// Rebuild the circular layout from the galaxy snapshot (only when flagged dirty).
    pub fn rebuild_if_needed(&mut self, canvas_rect: egui::Rect) {
        if self.galaxy_needs_rebuild
            && let Some(galaxy) = &self.galaxy
        {
            let center = canvas_rect.center();
            let radius = (canvas_rect.width().min(canvas_rect.height()) * 0.35).max(50.0);
            let (planets, edges) = build_planets_and_edges_from_galaxy(galaxy, center, radius);
            self.planets = planets;
            self.edges = edges;
            self.galaxy_needs_rebuild = false;
        }
    }

    /// Rebuild the id → screen-position cache when the planet list changes.
    pub fn refresh_pos_cache(&mut self) {
        if self.cached_pos_by_id.is_empty() || self.cached_pos_by_id.len() != self.planets.len() {
            self.cached_pos_by_id.clear();
            for planet in &self.planets {
                self.cached_pos_by_id.insert(planet.id, planet.pos);
            }
        }
    }
}
