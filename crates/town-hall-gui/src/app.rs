use std::collections::VecDeque;
use std::time::Duration;

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::{
    SimulationCommand, SimulationSnapshot, SimulationSource, accent, entity_view, market_view,
    muted, world_view,
};

const HISTORY_LENGTH: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorTab {
    Entities,
    Market,
}

pub struct TownHallApp {
    source: Box<dyn SimulationSource>,
    snapshot: SimulationSnapshot,
    running: bool,
    ticks_per_second: f32,
    tick_budget: f64,
    last_clock: Option<f64>,
    selected_entity: Option<u128>,
    inspector_tab: InspectorTab,
    entity_search: String,
    desired_population: u32,
    show_entity_labels: bool,
    show_goods: bool,
    show_resources: bool,
    grid_opacity: f32,
    zoom: f32,
    pan: Vec2,
    money_history: VecDeque<f64>,
    inventory_history: VecDeque<f64>,
}

impl TownHallApp {
    pub fn new(cc: &eframe::CreationContext<'_>, source: Box<dyn SimulationSource>) -> Self {
        configure_style(&cc.egui_ctx);
        Self {
            source,
            snapshot: SimulationSnapshot::loading(),
            running: false,
            ticks_per_second: 2.0,
            tick_budget: 0.0,
            last_clock: None,
            selected_entity: None,
            inspector_tab: InspectorTab::Entities,
            entity_search: String::new(),
            desired_population: 96,
            show_entity_labels: false,
            show_goods: true,
            show_resources: true,
            grid_opacity: 0.28,
            zoom: 1.0,
            pan: Vec2::ZERO,
            money_history: VecDeque::new(),
            inventory_history: VecDeque::new(),
        }
    }

    fn receive_snapshot(&mut self) {
        let Some(snapshot) = self.source.latest() else {
            return;
        };
        if snapshot.iteration != self.snapshot.iteration || self.money_history.is_empty() {
            push_history(&mut self.money_history, snapshot.total_money as f64);
            push_history(&mut self.inventory_history, snapshot.total_inventory as f64);
        }
        if self
            .selected_entity
            .is_some_and(|id| !snapshot.entities.iter().any(|entity| entity.id == id))
        {
            self.selected_entity = None;
        }
        self.snapshot = snapshot;
    }

    fn schedule_ticks(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        let elapsed = self
            .last_clock
            .replace(now)
            .map_or(0.0, |last| (now - last).clamp(0.0, 0.25));
        if self.running {
            self.tick_budget += elapsed * f64::from(self.ticks_per_second);
            let ticks = self.tick_budget.floor().min(8.0) as u32;
            if ticks > 0 {
                self.tick_budget -= f64::from(ticks);
                self.source.send(SimulationCommand::Advance(ticks));
            }
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            self.tick_budget = 0.0;
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn top_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("top_bar")
            .default_size(58.0)
            .min_size(58.0)
            .max_size(58.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(14, 20, 28)))
            .show(root, |ui| {
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("◈").size(25.0).color(accent()));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("TOWN HALL").strong().size(17.0));
                        ui.label(RichText::new("SIMULATION CONSOLE").size(9.0).color(muted()));
                    });
                    ui.add_space(22.0);
                    ui.separator();
                    ui.add_space(10.0);

                    let run_label = if self.running {
                        "⏸  Pause"
                    } else {
                        "▶  Run"
                    };
                    if ui
                        .add_sized([92.0, 32.0], egui::Button::new(run_label))
                        .clicked()
                    {
                        self.running = !self.running;
                    }
                    if ui
                        .add_enabled(
                            !self.running,
                            egui::Button::new("Step").min_size(Vec2::new(58.0, 32.0)),
                        )
                        .clicked()
                    {
                        self.source.send(SimulationCommand::Advance(1));
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("SPEED").size(10.0).color(muted()));
                    ui.add(
                        egui::Slider::new(&mut self.ticks_per_second, 0.25..=60.0)
                            .logarithmic(true)
                            .show_value(false),
                    );
                    ui.label(
                        RichText::new(format!("{:>4.1}×", self.ticks_per_second))
                            .monospace()
                            .color(accent()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        status_pill(ui, &self.snapshot.backend);
                        ui.label(
                            RichText::new(format!("TICK {}", compact(self.snapshot.iteration)))
                                .monospace()
                                .strong(),
                        );
                    });
                });
            });
    }

    fn settings_panel(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("settings")
            .default_size(226.0)
            .min_size(226.0)
            .max_size(226.0)
            .resizable(false)
            .frame(panel_frame())
            .show(root, |ui| {
                section_heading(ui, "WORLD LAYERS");
                ui.checkbox(&mut self.show_entity_labels, "Entity labels");
                ui.checkbox(&mut self.show_goods, "Inventory goods");
                ui.checkbox(&mut self.show_resources, "Resource deposits");
                ui.add_space(8.0);
                ui.label(RichText::new("Grid contrast").color(muted()));
                ui.add(egui::Slider::new(&mut self.grid_opacity, 0.05..=0.7));

                ui.add_space(18.0);
                section_heading(ui, "VIEW");
                ui.horizontal(|ui| {
                    if ui.button("−").clicked() {
                        self.zoom = (self.zoom / 1.2).max(0.5);
                    }
                    ui.label(format!("{:>3.0}%", self.zoom * 100.0));
                    if ui.button("+").clicked() {
                        self.zoom = (self.zoom * 1.2).min(12.0);
                    }
                    if ui.button("Fit").clicked() {
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                    }
                });
                ui.label(
                    RichText::new("Drag the map to pan · scroll to zoom")
                        .small()
                        .color(muted()),
                );

                ui.add_space(18.0);
                section_heading(ui, "NEW SESSION");
                ui.label(RichText::new("Population").color(muted()));
                ui.add(
                    egui::DragValue::new(&mut self.desired_population)
                        .range(1..=2_000)
                        .speed(1),
                );
                if ui
                    .add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new("Reset simulation"),
                    )
                    .clicked()
                {
                    self.running = false;
                    self.selected_entity = None;
                    self.money_history.clear();
                    self.inventory_history.clear();
                    self.source.send(SimulationCommand::Reset {
                        population: self.desired_population,
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} × {} tiles",
                            self.snapshot.width, self.snapshot.height
                        ))
                        .small()
                        .color(muted()),
                    );
                    ui.label(RichText::new(&self.snapshot.status).small().color(
                        if self.snapshot.error.is_some() {
                            Color32::from_rgb(255, 115, 115)
                        } else {
                            muted()
                        },
                    ));
                });
            });
    }

    fn inspector(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .default_size(300.0)
            .min_size(260.0)
            .max_size(390.0)
            .frame(panel_frame())
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.inspector_tab,
                        InspectorTab::Entities,
                        "Entities",
                    );
                    ui.selectable_value(
                        &mut self.inspector_tab,
                        InspectorTab::Market,
                        "Goods & activity",
                    );
                });
                ui.separator();
                match self.inspector_tab {
                    InspectorTab::Entities => {
                        if let Some(selection) = entity_view::show(
                            ui,
                            &self.snapshot,
                            self.selected_entity,
                            &mut self.entity_search,
                        ) {
                            self.selected_entity = selection;
                        }
                    }
                    InspectorTab::Market => market_view::show(ui, &self.snapshot),
                }
            });
    }

    fn central_view(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(9, 14, 20)))
            .show(root, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    stat_card(
                        ui,
                        "POPULATION",
                        compact(self.snapshot.entities.len() as u128),
                        "active entities",
                        None,
                    );
                    stat_card(
                        ui,
                        "TOTAL MONEY",
                        compact(self.snapshot.total_money),
                        "closed-system supply",
                        Some(&self.money_history),
                    );
                    stat_card(
                        ui,
                        "GOODS HELD",
                        compact(self.snapshot.total_inventory),
                        &format!("{} registered in use", self.snapshot.goods.len()),
                        Some(&self.inventory_history),
                    );
                    stat_card(
                        ui,
                        "WORLD MATERIAL",
                        compact(self.snapshot.total_material),
                        "primitive units",
                        None,
                    );
                });
                ui.add_space(10.0);
                let options = world_view::WorldViewOptions {
                    labels: self.show_entity_labels,
                    goods: self.show_goods,
                    resources: self.show_resources,
                    grid_opacity: self.grid_opacity,
                };
                if let Some(id) = world_view::show(
                    ui,
                    &self.snapshot,
                    self.selected_entity,
                    &options,
                    &mut self.zoom,
                    &mut self.pan,
                ) {
                    self.selected_entity = Some(id);
                    self.inspector_tab = InspectorTab::Entities;
                }
            });
    }
}

impl eframe::App for TownHallApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_snapshot();
        self.schedule_ticks(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.top_bar(ui);
        self.settings_panel(ui);
        self.inspector(ui);
        self.central_view(ui);
    }
}

fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(12, 18, 25);
    visuals.window_fill = Color32::from_rgb(14, 21, 29);
    visuals.extreme_bg_color = Color32::from_rgb(7, 11, 16);
    visuals.selection.bg_fill = Color32::from_rgb(39, 94, 79);
    visuals.selection.stroke.color = accent();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(27, 39, 50);
    visuals.widgets.active.bg_fill = Color32::from_rgb(34, 51, 63);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(12, 18, 25))
        .inner_margin(14.0)
}

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.0).strong().color(accent()));
    ui.add_space(2.0);
}

fn status_pill(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(19, 42, 38))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.0).color(accent()));
        });
}

fn stat_card(
    ui: &mut egui::Ui,
    title: &str,
    value: String,
    detail: &str,
    history: Option<&VecDeque<f64>>,
) {
    let available = ui.available_width();
    let width = ((available - 24.0) / 4.0).max(118.0);
    egui::Frame::new()
        .fill(Color32::from_rgb(15, 23, 31))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(27, 40, 52)))
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_width(width - 20.0);
            ui.label(RichText::new(title).size(9.0).color(muted()));
            ui.horizontal(|ui| {
                ui.label(RichText::new(value).size(21.0).strong().monospace());
                if let Some(history) = history {
                    sparkline(ui, history);
                }
            });
            ui.label(RichText::new(detail).size(9.0).color(muted()));
        });
}

fn sparkline(ui: &mut egui::Ui, values: &VecDeque<f64>) {
    let (response, painter) = ui.allocate_painter(Vec2::new(52.0, 22.0), egui::Sense::hover());
    if values.len() < 2 {
        return;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let spread = (max - min).max(1.0);
    let last = (values.len() - 1) as f32;
    let points = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            egui::pos2(
                egui::lerp(response.rect.x_range(), index as f32 / last),
                egui::lerp(
                    response.rect.y_range(),
                    1.0 - ((*value - min) / spread) as f32,
                ),
            )
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, accent())));
}

fn push_history(history: &mut VecDeque<f64>, value: f64) {
    if history.len() == HISTORY_LENGTH {
        history.pop_front();
    }
    history.push_back(value);
}

pub(crate) fn compact(value: u128) -> String {
    const SUFFIXES: [(u128, &str); 4] = [
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "K"),
    ];
    for (divisor, suffix) in SUFFIXES {
        if value >= divisor {
            return format!("{:.1}{suffix}", value as f64 / divisor as f64);
        }
    }
    value.to_string()
}
