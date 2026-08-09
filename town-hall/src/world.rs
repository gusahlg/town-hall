use crate::market::ResourceQuantity;
use crate::EntityId;
use crate::GoodQuantity;

pub struct World {
    tiles: Vec<Vec<Tile>>,
}

impl World {
    pub fn new(columns: usize, rows: usize) -> Self {
        let tiles = (0..rows)
            .map(|_| {
                (0..columns)
                    .map(|_| Tile::default())
                    .collect()
            })
            .collect();

        Self { tiles }
    }
}

pub struct Tile {
    resources: Vec<ResourceQuantity>,
    inhabitants: Vec<EntityId>,
    goods: Vec<GoodQuantity>,
}
impl Tile {
    pub fn new(resources: Vec<ResourceQuantity>) -> Self {
        Self {
            resources,
            inhabitants: Vec::new(),
            goods: Vec::new(),
        }
    }
}
impl Default for Tile {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
