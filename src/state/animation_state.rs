use common_game::utils::ID;
use std::collections::HashMap;
use std::time::Instant;

pub struct AnimationState {
    pub sending_asteroid: Option<(ID, Instant)>,
    pub sending_sunray: Option<(ID, Instant)>,
    pub planets_to_refresh: Vec<(ID, Instant)>,
    pub planet_displayed_charged: HashMap<ID, f32>,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            sending_asteroid: None,
            sending_sunray: None,
            planets_to_refresh: Vec::new(),
            planet_displayed_charged: HashMap::new(),
        }
    }

    /// Returns `true` when there are in-flight visual animations (asteroid /
    /// sunray effects or pending refresh requests) that need fast repaints.
    ///
    /// NOTE: `planet_displayed_charged` (the smooth counter animation) is
    /// intentionally excluded — it is a subtle cosmetic effect that can
    /// piggyback on repaints triggered by other sources instead of driving
    /// a continuous 60 fps loop on its own.
    pub fn has_active_animations(&self) -> bool {
        self.sending_asteroid.is_some()
            || self.sending_sunray.is_some()
            || !self.planets_to_refresh.is_empty()
    }
}
