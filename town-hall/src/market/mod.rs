//! This file describes all the mechanics regarding markets and trade.
mod resources;
pub use resources::*;

mod goods;
pub use goods::*;

mod recipes;
pub use recipes::*;
pub struct Market {
    // All of the different resources stored immutably
    pub resources: Resources,

    // Each element in good corresponds to the element in recipes at the same index, super easy
    // look-up
    pub goods: Vec<GoodEntry>,
}
impl Market {
    pub fn new() -> Self {
        Self { resources: Resources::default(), goods: Vec::new() }
    }
}
impl Default for Market {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GoodEntry {
    pub good: Good,
    pub recipes: Vec<Recipe>,
}
