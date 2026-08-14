use crate::{Quantity, ResourceId};

pub const DEFAULT_WORLD_WIDTH: u32 = 150;
pub const DEFAULT_WORLD_HEIGHT: u32 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

impl Position {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDeposit {
    pub resource: ResourceId,
    pub quantity: Quantity,
}

#[derive(Debug, Default)]
pub struct Tile {
    deposits: Vec<ResourceDeposit>,
}

impl Tile {
    #[inline]
    pub fn deposits(&self) -> &[ResourceDeposit] {
        &self.deposits
    }

    #[inline]
    pub fn deposits_mut(&mut self) -> &mut Vec<ResourceDeposit> {
        &mut self.deposits
    }
}

#[derive(Debug)]
pub struct World {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
}

impl World {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "world dimensions must be positive");
        let tile_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .expect("world dimensions overflow addressable memory");
        Self {
            width,
            height,
            tiles: (0..tile_count).map(|_| Tile::default()).collect(),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn contains(&self, position: Position) -> bool {
        position.x < self.width && position.y < self.height
    }

    pub fn tile(&self, position: Position) -> Option<&Tile> {
        self.index(position).map(|index| &self.tiles[index])
    }

    pub fn tile_mut(&mut self, position: Position) -> Option<&mut Tile> {
        self.index(position).map(|index| &mut self.tiles[index])
    }

    pub fn total_deposited_material(&self) -> u128 {
        self.tiles
            .iter()
            .flat_map(|tile| tile.deposits())
            .map(|deposit| deposit.quantity as u128)
            .sum()
    }

    fn index(&self, position: Position) -> Option<usize> {
        self.contains(position)
            .then(|| position.y as usize * self.width as usize + position.x as usize)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(DEFAULT_WORLD_WIDTH, DEFAULT_WORLD_HEIGHT)
    }
}
