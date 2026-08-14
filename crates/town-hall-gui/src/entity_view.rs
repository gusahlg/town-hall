use eframe::egui::{self, Color32, RichText};

use crate::{ActionKind, EntityKind, SimulationSnapshot, accent, muted};

pub(crate) fn show(
    ui: &mut egui::Ui,
    snapshot: &SimulationSnapshot,
    selected: Option<u128>,
    search: &mut String,
) -> Option<Option<u128>> {
    let mut new_selection = None;
    if let Some(entity) =
        selected.and_then(|id| snapshot.entities.iter().find(|entity| entity.id == id))
    {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("ENTITY #{}", entity.id)).strong());
            if ui.small_button("×").clicked() {
                new_selection = Some(None);
            }
        });
        ui.label(
            RichText::new(match entity.kind {
                EntityKind::Person => "Person",
                EntityKind::Company => "Company",
            })
            .color(muted()),
        );
        ui.add_space(8.0);
        detail_row(ui, "Money", &entity.money.to_string());
        detail_row(ui, "Position", &format!("{}, {}", entity.x, entity.y));
        detail_row(ui, "Inventory", &entity.inventory_units().to_string());
        ui.add_space(8.0);
        let action_color = match entity.action {
            ActionKind::Conserve => Color32::from_rgb(88, 171, 214),
            ActionKind::Predict => Color32::from_rgb(227, 185, 86),
            ActionKind::Give => accent(),
        };
        ui.label(RichText::new("LAST ACTION").size(9.0).color(muted()));
        ui.label(RichText::new(&entity.action_label).color(action_color));
        ui.add_space(8.0);
        ui.label(RichText::new("GOODS").size(9.0).color(muted()));
        if entity.inventory.is_empty() {
            ui.label(RichText::new("No goods held").italics().color(muted()));
        } else {
            for holding in &entity.inventory {
                detail_row(
                    ui,
                    &format!("Good #{}", holding.good),
                    &format!("{} units", holding.quantity),
                );
            }
        }
        ui.separator();
    }

    ui.add(
        egui::TextEdit::singleline(search)
            .hint_text("Filter by entity id…")
            .desired_width(f32::INFINITY),
    );
    let query = search.trim();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for entity in snapshot
                .entities
                .iter()
                .filter(|entity| query.is_empty() || entity.id.to_string().contains(query))
            {
                let is_selected = selected == Some(entity.id);
                let response = ui.selectable_label(is_selected, format!("#{}", entity.id));
                if response.clicked() {
                    new_selection = Some(Some(entity.id));
                }
                let action = match entity.action {
                    ActionKind::Conserve => "conserve",
                    ActionKind::Predict => "predict",
                    ActionKind::Give => "give",
                };
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("{} cr · {action}", entity.money))
                            .small()
                            .color(muted()),
                    );
                });
            }
        });
    new_selection
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(muted()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).monospace());
        });
    });
}
