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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Predict { amount: u64, target: EntityId },
    Conserve,
    Give { amount: u64, target: EntityId },
}

#[derive(Debug, Clone)]
pub struct Entity {
    kind: EntityKind,
    resources: u64,
    owners: Option<EntityId>,
    id: EntityId,
    last_action: Action,
}

impl Entity {
    pub fn new(kind: EntityKind, resources: u64, owners: Option<EntityId>, id: EntityId) -> Self {
        Self {
            kind,
            resources,
            owners,
            id,
            last_action: Action::Conserve,
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
    pub fn resources(&self) -> u64 {
        self.resources
    }

    #[inline]
    pub fn owners(&self) -> Option<EntityId> {
        self.owners
    }

    #[inline]
    pub fn last_action(&self) -> Action {
        self.last_action
    }

    pub(crate) fn set_resources(&mut self, resources: u64) {
        self.resources = resources;
    }

    pub(crate) fn set_last_action(&mut self, action: Action) {
        self.last_action = action;
    }
}
