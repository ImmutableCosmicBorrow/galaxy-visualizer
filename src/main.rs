use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use orchestrator::ExplorerType;
use orchestrator::planet::PlanetMap;
use orchestrator::ui::{OrchestratorToUiUpdate, UiToOrchestratorCommand};
use std::collections::{HashMap, HashSet};
mod manage_planets;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    println!("About to call run_native");
    eframe::run_native(
        "Immutable Cosmic Borrow Galaxy",
        options,
        Box::new(|cc| Ok(Box::new(GalaxyApp::new(cc)))),
    )
}

struct Planet {
    id: ID,
    pos: egui::Pos2,      // Where it is on screen
    color: egui::Color32, // To distinguish types (or use different image IDs)
    name: String,
}

struct GalaxyApp {
    galaxy: Option<PlanetMap>,
    planets: Vec<Planet>,
    edges: Vec<(ID, ID)>,
    cmd_sender: Sender<UiToOrchestratorCommand>,
    update_receiver: Receiver<OrchestratorToUiUpdate>,
    pending_spawn_pos: Option<egui::Pos2>,
}

impl GalaxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut planets = Vec::new();

        let (mut orch, cmd_sender, update_receiver) =
            orchestrator::create_with_path("galaxy/test_galaxy.txt", ExplorerType::Nico, None, None, 1000);

        cmd_sender
            .send(UiToOrchestratorCommand::GetGalaxy)
            .expect("Failed to send initial GetGalaxy command");

        cmd_sender
            .send(UiToOrchestratorCommand::GetExplorersPosition)
            .expect("Failed to send initial GetExplorerPosition command");

        cmd_sender
            .send(UiToOrchestratorCommand::AddExplorer(orchestrator::ExplorerType::Nico, 49153))
            .expect("Failed to send initial StartAllPlanetAIs command");

        std::thread::spawn(move || {
            orch.run();
            println!("Orchestrator created!");

            let tick = std::time::Duration::from_millis(16);
            loop {
                std::thread::sleep(tick);
            }
        });

        // Settings for the initial circle
        let center = egui::Pos2::new(400.0, 300.0); // Approx center of window
        let radius = 200.0;
        let count = 7;

        for i in 0..count {
            // Math to find x,y on a circle
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let pos = egui::Pos2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );

            planets.push(Planet {
                id: (i as ID) + 1,
                pos,
                color: egui::Color32::from_rgb(100, 200, 255), // Placeholder Blue
                name: format!("Planet {}", i + 1),
            });
        }

        Self {
            galaxy: None,
            planets,
            edges: Vec::new(),
            cmd_sender,
            update_receiver,
            pending_spawn_pos:None
        }
    }
}

fn canonical_edge(a: ID, b: ID) -> Option<(ID, ID)> {
    if a == b {
        None
    } else if a < b {
        Some((a, b))
    } else {
        Some((b, a))
    }
}

// Map the orchestrator's galaxy snapshot into renderable planets + edges for egui
fn build_planets_and_edges_from_galaxy(
    galaxy: &PlanetMap,
    center: egui::Pos2,
    radius: f32,
) -> (Vec<Planet>, Vec<(ID, ID)>) {
    let guard = galaxy
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mut ids: Vec<ID> = guard.keys().copied().collect();
    ids.sort();

    let count = ids.len().max(1) as f32;
    let mut planets = Vec::with_capacity(ids.len());

    for (i, id) in ids.iter().enumerate() {
        let angle = (i as f32 / count) * std::f32::consts::TAU;
        let pos = egui::Pos2::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        );

        planets.push(Planet {
            id: *id,
            pos,
            color: egui::Color32::from_rgb(100, 200, 255),
            name: format!("Planet {id}"),
        });
    }

    let mut edges = HashSet::new();
    for (id, node) in guard.iter() {
        for neighbor in node.neighbors_snapshot() {
            if let Some(edge) = canonical_edge(*id, neighbor) {
                edges.insert(edge);
            }
        }
    }

    (planets, edges.into_iter().collect())
}

impl eframe::App for GalaxyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Check for updates from orchestrator thread
            while let Ok(cmd) = self.update_receiver.try_recv() {
                match cmd {
                    OrchestratorToUiUpdate::Galaxy(galaxy) => {
                        self.galaxy = Some(galaxy);
                    }
                    OrchestratorToUiUpdate::DeadPlanet(_id) => {}
                    OrchestratorToUiUpdate::ExplorersPosition(_positions) => {}
                    OrchestratorToUiUpdate::PlanetSnapshot(_id, _snapshot) => {}
                    OrchestratorToUiUpdate::ExplorerSnapshot(_id, _bag) => {}
                    OrchestratorToUiUpdate::AutoMoveExplorer(_explorer_id, _from_id, _to_id) => {}

                    //maybe redundant? tanto devo refreshare la bag
                    OrchestratorToUiUpdate::AutoExplorerCraftsRes(_explorer_id, _resource) => {}
                    OrchestratorToUiUpdate::AutoExplorerCombineRes(_explorer_id, _resource) => {}

                    //draw supported combinations/resources
                    OrchestratorToUiUpdate::SupportedCombinations(_explorer_id, _combinations) => {}
                    OrchestratorToUiUpdate::SupportedResources(_explorer_id, _resources) => {}

                    //just draw sunray/asteroid
                    OrchestratorToUiUpdate::SendAutoSunray(_planet_id) => {}
                    OrchestratorToUiUpdate::SendAutoAsteroid(_planet_id) => {}

                    //last thing to do
                    OrchestratorToUiUpdate::StartPlanetAI(_planet_id) => {}
                    OrchestratorToUiUpdate::StopPlanetAI(_planet_id) => {}
                    OrchestratorToUiUpdate::ResetPlanetAI(_planet_id) => {}
                    OrchestratorToUiUpdate::StartExplorerAI(_explorer_id) => {}
                    OrchestratorToUiUpdate::StopExplorerAI(_explorer_id) => {}
                    OrchestratorToUiUpdate::ResetExplorerAI(_explorer_id) => {}
                    OrchestratorToUiUpdate::KillExplorerAI(_explorer_id) => {}
                    OrchestratorToUiUpdate::KillPlanetAI(_planet_id) => {}
                }
            }

            // Ask orchestrator for the latest snapshot each frame (cheap command)
            let _ = self.cmd_sender.send(UiToOrchestratorCommand::GetGalaxy);

            // reserve the whole screen for painting
            let canvas_rect = ui.available_rect_before_wrap();
            if let Some(galaxy) = &self.galaxy {
                let center = canvas_rect.center();
                let radius = (canvas_rect.width().min(canvas_rect.height()) * 0.35).max(50.0);
                let (planets, edges) = build_planets_and_edges_from_galaxy(galaxy, center, radius);
                self.planets = planets;
                self.edges = edges;
            }
            let (response, painter) = ui.allocate_painter(canvas_rect.size(), egui::Sense::click());

            // draw space background drak blue
            painter.rect_filled(
                canvas_rect,
                0.0, // Corner rounding
                egui::Color32::from_rgb(10, 10, 25),
            );

            // right click to add a planet -> open small floating menu (egui::Area)
            if response.clicked_by(egui::PointerButton::Secondary) {
                // prefer the precise interact pointer position, else fall back to hover
                let pos = response.interact_pointer_pos().or_else(|| ctx.input(|i| i.pointer.hover_pos()));
                self.pending_spawn_pos = pos;
                if let Some(p) = pos {
                    println!("Right-click at {:?}", p);
                } else {
                    println!("Right-click but pointer pos unavailable");
                }
            }

            if let Some(pos) = self.pending_spawn_pos {
                let mut chosen_id: Option<ID> = None;
                // Anchor a small floating area at the click position
                egui::Area::new(egui::Id::new("planet_menu_area"))
                    .fixed_pos(pos)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
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
                        });
                    });

                if let Some(new_id) = chosen_id {
                    self.planets.push(Planet {
                        id: new_id,
                        pos,
                        color: egui::Color32::from_rgb(160, 200, 240),
                        name: new_id.to_string(),
                    });

                    self.cmd_sender
                        .send(UiToOrchestratorCommand::AddPlanet(
                            new_id,
                            Vec::new(),
                        ))
                        .expect("Failed to send SpawnPlanet command");

                    self.pending_spawn_pos = None;
                }
            }

            // draw edges (from neighbors_snapshot)
            let mut pos_by_id: HashMap<ID, egui::Pos2> = HashMap::new();
            for planet in &self.planets {
                pos_by_id.insert(planet.id, planet.pos);
            }
            for (a, b) in &self.edges {
                if let (Some(pa), Some(pb)) = (pos_by_id.get(a), pos_by_id.get(b)) {
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

                // TODO: use Kenney asset
                /*
                painter.image(
                    my_planet_texture_id,
                    egui::Rect::from_center_size(planet.pos, egui::Vec2::splat(50.0)),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    planet.color // Tinting!
                );
                */

                
                painter.circle_filled(planet.pos, radius, planet.color);

                // draw name label centered below planet
                painter.text(
                    planet.pos + egui::Vec2::new(0.0, radius + 10.0),
                    egui::Align2::CENTER_TOP,
                    &planet.name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );
            }

            // instructions for the user to draw new planet
            painter.text(
                canvas_rect.min + egui::Vec2::new(20.0, 20.0),
                egui::Align2::LEFT_TOP,
                "Right-click anywhere to spawn a planet.",
                egui::FontId::monospace(14.0),
                egui::Color32::YELLOW,
            );

            // don't block the UI thread here
        });
    }
}
