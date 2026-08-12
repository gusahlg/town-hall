use crate::{Inventory, Need, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u128);

impl EntityId {
    pub(crate) const VACANT: Self = Self(u128::MAX);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Company,
}

/// Temporary action vocabulary. The neural/social layout is stable, but these
/// economic choices will be replaced once the decision model is designed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Predict { amount: u64, target: EntityId },
    Conserve,
    Give { amount: u64, target: EntityId },
}

pub type Money = u64;

#[derive(Debug, Clone)]
pub struct Entity {
    kind: EntityKind,
    inventory: Inventory,
    money: Money,
    location: Position,
    id: EntityId,
    last_action: Action,
    needs: Vec<Need>,
}

impl Entity {
    pub(crate) fn new(
        kind: EntityKind,
        inventory: Inventory,
        money: Money,
        location: Position,
        needs: Vec<Need>,
        id: EntityId,
    ) -> Self {
        Self {
            kind,
            inventory,
            money,
            location,
            id,
            last_action: Action::Conserve,
            needs,
        }
    }

    #[inline]
    pub fn id(&self) -> EntityId {
        self.id
    }

    #[inline]
    pub fn kind(&self) -> EntityKind {
        self.kind
    }

    #[inline]
    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    #[inline]
    pub fn money(&self) -> Money {
        self.money
    }

    #[inline]
    pub fn location(&self) -> Position {
        self.location
    }

    #[inline]
    pub fn needs(&self) -> &[Need] {
        &self.needs
    }

    #[inline]
    pub fn last_action(&self) -> Action {
        self.last_action
    }

    pub(crate) fn set_money(&mut self, money: Money) {
        self.money = money;
    }

    pub(crate) fn set_last_action(&mut self, action: Action) {
        self.last_action = action;
    }
}
