use eframe::egui::{self, Color32, RichText};

use crate::{SimulationSnapshot, accent, app::compact, muted};

pub(crate) fn show(ui: &mut egui::Ui, snapshot: &SimulationSnapshot) {
    ui.label(
        RichText::new("ACTION MIX")
            .size(10.0)
            .strong()
            .color(accent()),
    );
    let total = snapshot.entities.len().max(1) as f32;
    action_bar(
        ui,
        "Conserve",
        snapshot.activity.conserve,
        total,
        Color32::from_rgb(88, 171, 214),
    );
    action_bar(
        ui,
        "Predict",
        snapshot.activity.predict,
        total,
        Color32::from_rgb(227, 185, 86),
    );
    action_bar(ui, "Give", snapshot.activity.give, total, accent());

    ui.add_space(16.0);
    ui.label(
        RichText::new("GOODS IN CIRCULATION")
            .size(10.0)
            .strong()
            .color(accent()),
    );
    if snapshot.goods.is_empty() {
        egui::Frame::new()
            .fill(Color32::from_rgb(16, 24, 32))
            .corner_radius(5.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(RichText::new("No goods are currently held.").color(muted()));
                ui.label(
                    RichText::new("The economic registries are ready for production rules.")
                        .small()
                        .color(muted()),
                );
            });
        return;
    }

    egui::Grid::new("goods_grid")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new("GOOD").small().color(muted()));
            ui.label(RichText::new("UNITS").small().color(muted()));
            ui.label(RichText::new("HOLDERS").small().color(muted()));
            ui.end_row();
            for good in &snapshot.goods {
                ui.label(format!("#{}", good.id));
                ui.label(RichText::new(compact(good.total_units)).monospace());
                ui.label(RichText::new(good.holders.to_string()).monospace());
                ui.end_row();
            }
        });

    ui.add_space(16.0);
    ui.label(
        RichText::new("MAP LEGEND")
            .size(10.0)
            .strong()
            .color(accent()),
    );
    legend(ui, Color32::from_rgb(88, 171, 214), "Conserving entity");
    legend(ui, Color32::from_rgb(227, 185, 86), "Predicting entity");
    legend(ui, accent(), "Giving entity");
    legend(ui, Color32::from_rgb(219, 118, 255), "Goods held");
}

fn action_bar(ui: &mut egui::Ui, label: &str, count: usize, total: f32, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(muted()));
        let fraction = count as f32 / total;
        ui.add(
            egui::ProgressBar::new(fraction)
                .desired_width(ui.available_width() - 38.0)
                .fill(color)
                .text(count.to_string()),
        );
    });
}

fn legend(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (response, painter) = ui.allocate_painter(egui::vec2(12.0, 12.0), egui::Sense::hover());
        painter.circle_filled(response.rect.center(), 4.0, color);
        ui.label(RichText::new(label).small().color(muted()));
    });
}
