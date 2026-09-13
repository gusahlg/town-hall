//! Reusable eframe presentation layer for Town Hall.
//!
//! The views consume snapshots and emit small commands. Native builds connect
//! that boundary to the real simulation on a worker thread, while web builds
//! use a lightweight deterministic session so the Vulkan simulation core can
//! remain a native-only library.

mod app;
mod entity_view;
mod market_view;
mod world_view;

pub use app::TownHallApp;

use eframe::egui;

/// Session defaults shared by every client. The web build cannot depend on
/// the native-only simulation crate, so these mirror `town_hall`'s constants;
/// the native module pins the pairing with compile-time asserts.
pub const DEFAULT_POPULATION: u32 = 50_000;
pub const DEFAULT_ENTITY_MONEY: u64 = 100;
pub const DEFAULT_WORLD_WIDTH: u32 = 150;
pub const DEFAULT_WORLD_HEIGHT: u32 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Company,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Conserve,
    Predict,
    Give,
}

#[derive(Debug, Clone)]
pub struct HoldingSnapshot {
    pub good: usize,
    pub quantity: u32,
}

#[derive(Debug, Clone)]
pub struct EntitySnapshot {
    pub id: u128,
    pub kind: EntityKind,
    pub x: u32,
    pub y: u32,
    pub money: u64,
    pub inventory: Vec<HoldingSnapshot>,
    pub action: ActionKind,
    pub action_label: String,
}

impl EntitySnapshot {
    pub fn inventory_units(&self) -> u128 {
        self.inventory
            .iter()
            .map(|holding| u128::from(holding.quantity))
            .sum()
    }
}

#[derive(Debug, Clone)]
pub struct GoodSnapshot {
    pub id: usize,
    pub total_units: u128,
    pub holders: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub x: u32,
    pub y: u32,
    pub resource: usize,
    pub quantity: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ActivitySnapshot {
    pub conserve: usize,
    pub predict: usize,
    pub give: usize,
}

#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub iteration: u128,
    pub width: u32,
    pub height: u32,
    pub total_money: u128,
    pub total_inventory: u128,
    pub total_material: u128,
    pub entities: Vec<EntitySnapshot>,
    pub goods: Vec<GoodSnapshot>,
    pub resources: Vec<ResourceSnapshot>,
    pub activity: ActivitySnapshot,
    pub backend: String,
    pub status: String,
    pub error: Option<String>,
}

impl SimulationSnapshot {
    fn loading() -> Self {
        Self {
            iteration: 0,
            width: DEFAULT_WORLD_WIDTH,
            height: DEFAULT_WORLD_HEIGHT,
            total_money: 0,
            total_inventory: 0,
            total_material: 0,
            entities: Vec::new(),
            goods: Vec::new(),
            resources: Vec::new(),
            activity: ActivitySnapshot::default(),
            backend: "Starting simulation…".into(),
            status: "Preparing the world".into(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SimulationCommand {
    Advance(u32),
    Reset { population: u32 },
}

pub trait SimulationSource {
    fn send(&mut self, command: SimulationCommand);
    fn latest(&mut self) -> Option<SimulationSnapshot>;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_app(cc: &eframe::CreationContext<'_>) -> TownHallApp {
    TownHallApp::new(cc, Box::new(native::NativeSimulation::new()))
}

#[cfg(target_arch = "wasm32")]
pub fn web_app(cc: &eframe::CreationContext<'_>) -> TownHallApp {
    TownHallApp::new(cc, Box::new(DemoSimulation::new(DEFAULT_POPULATION)))
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::collections::BTreeMap;
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use std::thread;

    use neuro_ash::{CpuEvaluator, Gpu};
    use town_hall::{Action, DEFAULT_RANDOM_SEED, EntityKind as ModelEntityKind, Village};

    use super::{
        ActionKind, ActivitySnapshot, DEFAULT_ENTITY_MONEY, DEFAULT_POPULATION,
        DEFAULT_WORLD_HEIGHT, DEFAULT_WORLD_WIDTH, EntityKind, EntitySnapshot, GoodSnapshot,
        HoldingSnapshot, ResourceSnapshot, SimulationCommand, SimulationSnapshot, SimulationSource,
    };

    const _: () = assert!(DEFAULT_POPULATION as usize == town_hall::STARTING_POPULATION);
    const _: () = assert!(DEFAULT_ENTITY_MONEY == town_hall::INITIAL_ENTITY_MONEY);
    const _: () = assert!(
        DEFAULT_WORLD_WIDTH == town_hall::DEFAULT_WORLD_WIDTH
            && DEFAULT_WORLD_HEIGHT == town_hall::DEFAULT_WORLD_HEIGHT
    );

    enum WorkerCommand {
        Simulation(SimulationCommand),
        Shutdown,
    }

    enum Evaluator {
        Gpu(Box<Gpu>),
        Cpu(CpuEvaluator),
    }

    impl Evaluator {
        fn step(&mut self, village: &mut Village) -> anyhow::Result<()> {
            match self {
                Self::Gpu(gpu) => village.step(gpu.as_mut()),
                Self::Cpu(cpu) => village.step(cpu),
            }
        }

        fn clear(&mut self) {
            if let Self::Gpu(gpu) = self {
                gpu.clear_cache();
            }
        }

        fn label(&self) -> String {
            match self {
                Self::Gpu(gpu) => format!("GPU · {}", gpu.device_name()),
                Self::Cpu(_) => "CPU evaluator".into(),
            }
        }
    }

    pub(super) struct NativeSimulation {
        commands: SyncSender<WorkerCommand>,
        snapshots: Receiver<SimulationSnapshot>,
    }

    impl NativeSimulation {
        pub(super) fn new() -> Self {
            let (command_tx, command_rx) = mpsc::sync_channel(4);
            let (snapshot_tx, snapshot_rx) = mpsc::channel();
            thread::Builder::new()
                .name("town-hall-simulation".into())
                .spawn(move || {
                    let mut evaluator = match Gpu::new() {
                        Ok(gpu) => Evaluator::Gpu(Box::new(gpu)),
                        Err(_) => Evaluator::Cpu(CpuEvaluator),
                    };
                    let mut village =
                        Village::with_population(DEFAULT_POPULATION as usize, DEFAULT_RANDOM_SEED);
                    let _ = snapshot_tx.send(snapshot(&village, evaluator.label(), None));

                    while let Ok(command) = command_rx.recv() {
                        match command {
                            WorkerCommand::Shutdown => break,
                            WorkerCommand::Simulation(SimulationCommand::Reset { population }) => {
                                evaluator.clear();
                                village = Village::with_population(
                                    population as usize,
                                    DEFAULT_RANDOM_SEED,
                                );
                                let _ =
                                    snapshot_tx.send(snapshot(&village, evaluator.label(), None));
                            }
                            WorkerCommand::Simulation(SimulationCommand::Advance(count)) => {
                                let mut error = None;
                                for _ in 0..count {
                                    if let Err(problem) = evaluator.step(&mut village) {
                                        error = Some(problem.to_string());
                                        break;
                                    }
                                }
                                let _ =
                                    snapshot_tx.send(snapshot(&village, evaluator.label(), error));
                            }
                        }
                    }
                })
                .expect("failed to start the simulation worker");
            Self {
                commands: command_tx,
                snapshots: snapshot_rx,
            }
        }
    }

    impl SimulationSource for NativeSimulation {
        fn send(&mut self, command: SimulationCommand) {
            let _ = self.commands.try_send(WorkerCommand::Simulation(command));
        }

        fn latest(&mut self) -> Option<SimulationSnapshot> {
            let mut latest = None;
            while let Ok(snapshot) = self.snapshots.try_recv() {
                latest = Some(snapshot);
            }
            latest
        }
    }

    impl Drop for NativeSimulation {
        fn drop(&mut self) {
            let _ = self.commands.try_send(WorkerCommand::Shutdown);
        }
    }

    fn snapshot(village: &Village, backend: String, error: Option<String>) -> SimulationSnapshot {
        let mut activity = ActivitySnapshot::default();
        let entities = village
            .entities()
            .iter()
            .map(|entity| {
                let (action, action_label) = match entity.last_action() {
                    Action::Conserve => {
                        activity.conserve += 1;
                        (ActionKind::Conserve, "Conserve".into())
                    }
                    Action::Predict { amount, target } => {
                        activity.predict += 1;
                        (
                            ActionKind::Predict,
                            format!("Predict {amount} → #{}", target.0),
                        )
                    }
                    Action::Give { amount, target } => {
                        activity.give += 1;
                        (ActionKind::Give, format!("Give {amount} → #{}", target.0))
                    }
                };
                EntitySnapshot {
                    id: entity.id().0,
                    kind: match entity.kind() {
                        ModelEntityKind::Person => EntityKind::Person,
                        ModelEntityKind::Company => EntityKind::Company,
                    },
                    x: entity.location().x,
                    y: entity.location().y,
                    money: entity.money(),
                    inventory: entity
                        .inventory()
                        .entries()
                        .map(|holding| HoldingSnapshot {
                            good: holding.good.index(),
                            quantity: holding.quantity,
                        })
                        .collect(),
                    action,
                    action_label,
                }
            })
            .collect::<Vec<_>>();

        let mut good_totals = BTreeMap::<usize, (u128, usize)>::new();
        for entity in &entities {
            for holding in &entity.inventory {
                let summary = good_totals.entry(holding.good).or_default();
                summary.0 += u128::from(holding.quantity);
                summary.1 += 1;
            }
        }
        let goods = good_totals
            .into_iter()
            .map(|(id, (total_units, holders))| GoodSnapshot {
                id,
                total_units,
                holders,
            })
            .collect();

        let world = village.world();
        let mut resources = Vec::new();
        for y in 0..world.height() {
            for x in 0..world.width() {
                if let Some(tile) = world.tile(town_hall::Position::new(x, y)) {
                    resources.extend(
                        tile.deposits()
                            .iter()
                            .flat_map(|deposit| &deposit.resources)
                            .map(|info| ResourceSnapshot {
                                x,
                                y,
                                resource: info.resource_id.index(),
                                quantity: info.quantity,
                            }),
                    );
                }
            }
        }

        SimulationSnapshot {
            iteration: village.iterations(),
            width: world.width(),
            height: world.height(),
            total_money: village.total_money(),
            total_inventory: entities.iter().map(EntitySnapshot::inventory_units).sum(),
            total_material: world.total_deposited_material(),
            entities,
            goods,
            resources,
            activity,
            backend,
            status: if error.is_some() {
                "Simulation stopped after an evaluator error".into()
            } else {
                "Live model".into()
            },
            error,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use town_hall::{
            DepositProperties, Position, ResourceDeposit, ResourceId, ResourceInfo, World,
        };

        #[test]
        fn snapshot_includes_every_resource_in_mixed_deposits() {
            let mut village = Village::with_population(0, 42);
            *village.world_mut() = World::empty(2, 1);
            let deposit = ResourceDeposit {
                resources: [(7, 11), (203, 17)]
                    .into_iter()
                    .map(|(id, quantity)| ResourceInfo {
                        resource_id: ResourceId(id),
                        quantity,
                        recovery_rate: 0,
                    })
                    .collect(),
                properties: DepositProperties {
                    time: 1,
                    parallelisation: 1,
                    gear: 0,
                    efficiency: 100,
                    batch_size: 1,
                },
            };
            for x in 0..2 {
                village
                    .world_mut()
                    .tile_mut(Position::new(x, 0))
                    .unwrap()
                    .deposits_mut()
                    .push(deposit.clone());
            }
            let snapshot = snapshot(&village, "CPU".into(), None);
            let actual: Vec<_> = snapshot
                .resources
                .iter()
                .map(|info| (info.x, info.y, info.resource, info.quantity))
                .collect();
            assert_eq!(
                actual,
                [
                    (0, 0, 7, 11),
                    (0, 0, 203, 17),
                    (1, 0, 7, 11),
                    (1, 0, 203, 17)
                ]
            );
            assert_eq!(snapshot.total_material, 56);
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct DemoSimulation {
    snapshot: SimulationSnapshot,
    dirty: bool,
}

#[cfg(target_arch = "wasm32")]
impl DemoSimulation {
    fn new(population: u32) -> Self {
        let mut simulation = Self {
            snapshot: SimulationSnapshot::loading(),
            dirty: true,
        };
        simulation.reset(population);
        simulation
    }

    /// Mirrors the native population's starting money, empty inventories, and
    /// spawn order. The browser preview does not run the native world generator.
    fn reset(&mut self, population: u32) {
        let tile_count = DEFAULT_WORLD_WIDTH * DEFAULT_WORLD_HEIGHT;
        self.snapshot = SimulationSnapshot {
            iteration: 0,
            width: DEFAULT_WORLD_WIDTH,
            height: DEFAULT_WORLD_HEIGHT,
            total_money: 0,
            total_inventory: 0,
            total_material: 0,
            entities: (0..population)
                .map(|id| {
                    let tile = id % tile_count;
                    EntitySnapshot {
                        id: u128::from(id),
                        kind: EntityKind::Person,
                        x: tile % DEFAULT_WORLD_WIDTH,
                        y: tile / DEFAULT_WORLD_WIDTH,
                        money: DEFAULT_ENTITY_MONEY,
                        inventory: Vec::new(),
                        action: ActionKind::Conserve,
                        action_label: "Conserve".into(),
                    }
                })
                .collect(),
            goods: Vec::new(),
            resources: Vec::new(),
            activity: ActivitySnapshot::default(),
            backend: "WebAssembly · local preview".into(),
            status: "Interactive browser session".into(),
            error: None,
        };
        self.recalculate();
    }

    fn advance(&mut self) {
        self.snapshot.iteration += 1;
        let tick = self.snapshot.iteration;
        let population = self.snapshot.entities.len().max(1) as u128;
        for entity in &mut self.snapshot.entities {
            let phase = (entity.id + tick) % 11;
            entity.x = (entity.x + u32::from(phase == 2)) % self.snapshot.width;
            entity.y = (entity.y + u32::from(phase == 7)) % self.snapshot.height;
            match phase {
                0..=5 => {
                    entity.action = ActionKind::Conserve;
                    entity.action_label = "Conserve".into();
                }
                6..=8 => {
                    entity.action = ActionKind::Predict;
                    entity.action_label = format!("Predict → #{}", (entity.id + 7) % population);
                }
                _ => {
                    entity.action = ActionKind::Give;
                    entity.action_label = format!("Give 1 → #{}", (entity.id + 1) % population);
                    entity.money = entity.money.saturating_sub(1);
                }
            }
        }
        self.recalculate();
    }

    fn recalculate(&mut self) {
        use std::collections::BTreeMap;

        self.snapshot.total_money = self
            .snapshot
            .entities
            .iter()
            .map(|entity| u128::from(entity.money))
            .sum();
        self.snapshot.total_inventory = self
            .snapshot
            .entities
            .iter()
            .map(EntitySnapshot::inventory_units)
            .sum();
        self.snapshot.activity = ActivitySnapshot::default();
        let mut goods = BTreeMap::<usize, (u128, usize)>::new();
        for entity in &self.snapshot.entities {
            match entity.action {
                ActionKind::Conserve => self.snapshot.activity.conserve += 1,
                ActionKind::Predict => self.snapshot.activity.predict += 1,
                ActionKind::Give => self.snapshot.activity.give += 1,
            }
            for holding in &entity.inventory {
                let summary = goods.entry(holding.good).or_default();
                summary.0 += u128::from(holding.quantity);
                summary.1 += 1;
            }
        }
        self.snapshot.goods = goods
            .into_iter()
            .map(|(id, (total_units, holders))| GoodSnapshot {
                id,
                total_units,
                holders,
            })
            .collect();
    }
}

#[cfg(target_arch = "wasm32")]
impl SimulationSource for DemoSimulation {
    fn send(&mut self, command: SimulationCommand) {
        match command {
            SimulationCommand::Advance(count) => {
                for _ in 0..count {
                    self.advance();
                }
            }
            SimulationCommand::Reset { population } => self.reset(population),
        }
        self.dirty = true;
    }

    fn latest(&mut self) -> Option<SimulationSnapshot> {
        // Cloning the full population is only worthwhile when it changed.
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(self.snapshot.clone())
    }
}

pub(crate) fn accent() -> egui::Color32 {
    egui::Color32::from_rgb(89, 214, 174)
}

pub(crate) fn muted() -> egui::Color32 {
    egui::Color32::from_rgb(137, 151, 166)
}
