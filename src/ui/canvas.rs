use common_game::utils::ID;
use eframe::egui;
use std::collections::HashMap;
use std::fmt::Write as _;

use crate::helpers::format_bag_content;
use crate::models::Planet;
use crate::state::{AnimationState, ExplorerState, GalaxyState, UiState};

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

pub fn draw_background(painter: &egui::Painter, canvas_rect: egui::Rect) {
    // draw space background dark blue
    painter.rect_filled(
        canvas_rect,
        0.0, // Corner rounding
        egui::Color32::from_rgb(10, 10, 25),
    );
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

pub fn draw_edges(
    painter: &egui::Painter,
    edges: &[(ID, ID)],
    cached_pos_by_id: &HashMap<ID, egui::Pos2>,
) {
    for (a, b) in edges {
        if let (Some(pa), Some(pb)) = (cached_pos_by_id.get(a), cached_pos_by_id.get(b)) {
            painter.line_segment(
                [*pa, *pb],
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(30)),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Planets & explorers
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
pub fn draw_planets_and_explorers(
    ctx: &egui::Context,
    painter: &egui::Painter,
    response: &egui::Response,
    galaxy_state: &GalaxyState,
    explorer_state: &ExplorerState,
    animation_state: &mut AnimationState,
    ui_state: &mut UiState,
) {
    for planet in &galaxy_state.planets {
        draw_single_planet(
            ctx,
            painter,
            response,
            planet,
            &galaxy_state.planet_states,
            explorer_state,
            animation_state,
            ui_state,
        );
    }
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_arguments)]
fn draw_single_planet(
    ctx: &egui::Context,
    painter: &egui::Painter,
    response: &egui::Response,
    planet: &Planet,
    planet_states: &std::collections::HashMap<
        ID,
        common_game::components::planet::DummyPlanetState,
    >,
    explorer_state: &ExplorerState,
    animation_state: &mut AnimationState,
    ui_state: &mut UiState,
) {
    // size
    let radius = 20.0;

    // Check for right-click on this planet
    if let Some(pointer_pos) = response.interact_pointer_pos() {
        let distance = pointer_pos.distance(planet.pos);
        if distance <= radius && response.clicked_by(egui::PointerButton::Secondary) {
            ui_state.selected_planet = Some(planet.id);
            ui_state.context_menu_pos = Some(pointer_pos);
        }
    }

    // Draw colored shadow if planet is selected
    if ui_state.selected_planet == Some(planet.id) {
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
        painter.circle_filled(planet.pos, radius, egui::Color32::from_rgb(100, 200, 255));
    }

    // draw explorers on this planet as small colored dots
    draw_explorers_on_planet(
        ctx,
        painter,
        response,
        planet,
        radius,
        explorer_state,
        ui_state,
    );

    // draw name label centered below planet
    painter.text(
        planet.pos + egui::Vec2::new(0.0, radius + 10.0),
        egui::Align2::CENTER_TOP,
        &planet.name,
        egui::FontId::proportional(14.0),
        egui::Color32::WHITE,
    );

    // Draw planet state if available
    draw_planet_state(ctx, painter, planet, radius, planet_states, animation_state);
}

// ---------------------------------------------------------------------------
// Explorers on a single planet
// ---------------------------------------------------------------------------

#[allow(clippy::cast_precision_loss)]
fn draw_explorers_on_planet(
    _ctx: &egui::Context,
    painter: &egui::Painter,
    response: &egui::Response,
    planet: &Planet,
    planet_radius: f32,
    explorer_state: &ExplorerState,
    ui_state: &mut UiState,
) {
    let explorers_on_planet: Vec<ID> = explorer_state
        .explorer_positions
        .iter()
        .filter(|(_, pid)| **pid == planet.id)
        .map(|(eid, _)| *eid)
        .collect();

    for (i, explorer_id) in explorers_on_planet.iter().enumerate() {
        let angle = (i as f32 / explorers_on_planet.len().max(1) as f32) * std::f32::consts::TAU;
        let explorer_pos = planet.pos
            + egui::Vec2::new(
                (planet_radius + 12.0) * angle.cos(),
                (planet_radius + 12.0) * angle.sin(),
            );
        // Explorer visual parameters
        let explorer_radius = 8.0; // made larger per request

        // Check for right-click on explorer
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let distance = pointer_pos.distance(explorer_pos);
            if distance <= explorer_radius && response.clicked_by(egui::PointerButton::Secondary) {
                ui_state.selected_explorer = Some(*explorer_id);
                ui_state.selected_planet = None;
                ui_state.context_menu_pos = Some(pointer_pos);
            }
        }

        // Draw explorer as a larger bright yellow dot
        painter.circle_filled(explorer_pos, explorer_radius, egui::Color32::YELLOW);

        // Draw explorer label with name, ID, and bag content
        let explorer_name = orchestrator::id::IdManager::explorer_name_from_id(*explorer_id);
        let bag_line = match explorer_state.explorer_bags.get(explorer_id) {
            Some(bag) => format_bag_content(bag),
            None => "bag: loading".to_string(),
        };
        let label_text = format!("{explorer_name} {explorer_id}\n{bag_line}");
        painter.text(
            explorer_pos + egui::Vec2::new(explorer_radius + 6.0, -explorer_radius - 6.0),
            egui::Align2::LEFT_TOP,
            label_text,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
    }
}

// ---------------------------------------------------------------------------
// Planet state overlay (energy cells, rocket icon, …)
// ---------------------------------------------------------------------------

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation)]
fn draw_planet_state(
    ctx: &egui::Context,
    painter: &egui::Painter,
    planet: &Planet,
    radius: f32,
    planet_states: &std::collections::HashMap<
        ID,
        common_game::components::planet::DummyPlanetState,
    >,
    animation_state: &mut AnimationState,
) {
    if let Some(state) = planet_states.get(&planet.id) {
        let state_pos = planet.pos + egui::Vec2::new(0.0, radius + 28.0);

        // Animate displayed charged counter towards actual value
        let dt = ctx.input(|i| i.stable_dt);
        let target_charged = state.charged_cells_count as f32;
        let displayed = animation_state
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
        let _ = write!(
            state_text,
            "⚡{}/{}",
            displayed_charged_usize,
            state.energy_cells.len()
        );

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

// ---------------------------------------------------------------------------
// Help text
// ---------------------------------------------------------------------------

pub fn draw_help_text(painter: &egui::Painter, canvas_rect: egui::Rect) {
    painter.text(
        canvas_rect.min + egui::Vec2::new(20.0, 20.0),
        egui::Align2::LEFT_TOP,
        "Right-click a planet for options. Use the top-right button to create a planet.",
        egui::FontId::monospace(14.0),
        egui::Color32::YELLOW,
    );
}
