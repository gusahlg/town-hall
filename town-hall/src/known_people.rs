use crate::{EntityId, KNOWN_PERSON_COUNT, KNOWN_PERSON_MASK_WORDS};

#[derive(Debug)]
pub struct KnownPeoplePool {
    // `EntityId::VACANT` avoids the extra discriminant/padding of
    // `Option<EntityId>` across hundreds of slots per entity.
    occupants: Vec<EntityId>,
    active: Vec<[u64; KNOWN_PERSON_MASK_WORDS]>,
    rows: usize,
}

impl KnownPeoplePool {
    pub fn with_capacity(row_capacity: usize) -> Self {
        let occupant_capacity = row_capacity
            .checked_mul(KNOWN_PERSON_COUNT)
            .expect("known-person storage capacity overflows usize");
        Self {
            occupants: Vec::with_capacity(occupant_capacity),
            active: Vec::with_capacity(row_capacity),
            rows: 0,
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.rows
    }

    pub fn push_empty(&mut self) {
        self.occupants
            .extend([EntityId::VACANT; KNOWN_PERSON_COUNT]);
        self.active.push([0; KNOWN_PERSON_MASK_WORDS]);
        self.rows += 1;
    }

    pub fn shrink_to_fit(&mut self) {
        self.occupants.shrink_to_fit();
        self.active.shrink_to_fit();
    }

    pub fn swap_remove(&mut self, row: usize) {
        assert!(row < self.rows);
        let last = self.rows - 1;
        if row != last {
            let source = last * KNOWN_PERSON_COUNT;
            let destination = row * KNOWN_PERSON_COUNT;
            self.occupants
                .copy_within(source..source + KNOWN_PERSON_COUNT, destination);
        }
        self.occupants.truncate(last * KNOWN_PERSON_COUNT);
        self.active.swap_remove(row);
        self.rows -= 1;
    }

    #[inline]
    pub fn occupant(&self, row: usize, slot: usize) -> Option<EntityId> {
        let occupant = self.occupants[self.index(row, slot)];
        (occupant != EntityId::VACANT).then_some(occupant)
    }

    #[inline]
    pub fn is_active(&self, row: usize, slot: usize) -> bool {
        self.index(row, slot);
        self.active[row][slot / 64] & (1_u64 << (slot % 64)) != 0
    }

    pub(crate) fn set_active_mask(&mut self, row: usize, mask: [u64; KNOWN_PERSON_MASK_WORDS]) {
        assert!(row < self.rows);
        self.active[row] = mask;
    }

    pub(crate) fn replace(&mut self, row: usize, slot: usize, occupant: EntityId) {
        let index = self.index(row, slot);
        debug_assert!(occupant != EntityId::VACANT);
        self.occupants[index] = occupant;
        self.active[row][slot / 64] |= 1_u64 << (slot % 64);
    }

    fn index(&self, row: usize, slot: usize) -> usize {
        assert!(row < self.rows);
        assert!(slot < KNOWN_PERSON_COUNT);
        row * KNOWN_PERSON_COUNT + slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_fixed_width_and_swap_removed_together() {
        let mut pool = KnownPeoplePool::with_capacity(2);
        pool.push_empty();
        pool.push_empty();
        pool.replace(1, 17, EntityId(99));
        pool.swap_remove(0);

        assert_eq!(pool.len(), 1);
        assert_eq!(pool.occupant(0, 17), Some(EntityId(99)));
        assert!(pool.is_active(0, 17));
    }
}
