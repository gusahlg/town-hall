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
    preference: Preference,
}

impl Entity {
    pub fn new(kind: EntityKind, resources: u64, owners: Option<EntityId>, id: EntityId) -> Self {
        Self {
            kind,
            resources,
            owners,
            id,
            last_action: Action::Conserve,
            // Preference should be pseud-random and need should be updated regularly due to
            // different events.
            preference: Preference { preferences: vec![vec![55, 72, 241], vec![0, 32, 593, 45]], 
                needs: vec![Need {target: vec![0], intensity: 0.95, recovery_rate: 5.0 }, Need {target: vec![3, 8], intensity: 1.0, recovery_rate: 10.0 }] },
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

#[derive(Debug, Clone)]
struct Preference {
    // Each u16 maps to a resource in resources
    preferences: Vec<Vec<u16>>,
    pub needs: Vec<Need>,
}

#[derive(Debug, Clone)]
struct Need {
    target: Vec<u16>,
    intensity: f32, // 0..1
    recovery_rate: f32,
}
