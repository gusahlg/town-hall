//! This file defines what a good is.
use dimension_macro::dimensions;

// Defines the resource type as well as a DIMENSIONS constant
dimensions!(100);

pub struct Resources {
    pub resources: ResourcesType,
}

impl Resources {
    // NOTE: The new function should fill in the vectors layer by layer with 100 numbers in
    // sequential order carrying over to the next layer when a layer ends. Maybe these should not be
    // vectors but instead arrays becaue they are fixed sizes?
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    pub fn distance(&self, r1: u16, r2: u16) -> u16 {
        let r1 = self.get_position(r1);
        let r2 = self.get_position(r2);
        let distance = r1
            .into_iter()
            .zip(r2)
            .map(|(left, right)| left.abs_diff(right))
            .sum::<usize>();

        u16::try_from(distance).expect("resource distance does not fit in a u16")
    }

    fn get_position(&self, resource: u16) -> [usize; DIMENSIONS] {
        let mut position = [0; DIMENSIONS];

        self.resources
            .find_position(resource, &mut position, 0)
            .then_some(position)
            .unwrap_or_else(|| panic!("resource {resource} does not exist"))
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}
trait ResourceTree {
    fn find_position<const N: usize>(
        &self,
        resource: u16,
        position: &mut [usize; N],
        depth: usize,
    ) -> bool;
}

impl ResourceTree for u16 {
    fn find_position<const N: usize>(
        &self,
        resource: u16,
        _position: &mut [usize; N],
        depth: usize,
    ) -> bool {
        depth == N && *self == resource
    }
}

impl<T: ResourceTree> ResourceTree for Vec<T> {
    fn find_position<const N: usize>(
        &self,
        resource: u16,
        position: &mut [usize; N],
        depth: usize,
    ) -> bool {
        if depth == N {
            return false;
        }

        self.iter().enumerate().any(|(index, child)| {
            position[depth] = index;
            child.find_position(resource, position, depth + 1)
        })
    }
}

