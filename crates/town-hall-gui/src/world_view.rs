use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::{ActionKind, EntitySnapshot, SimulationSnapshot, accent, muted};

pub(crate) struct WorldViewOptions {
    pub labels: bool,
    pub goods: bool,
    pub resources: bool,
    pub grid_opacity: f32,
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    snapshot: &SimulationSnapshot,
    selected: Option<u128>,
    options: &WorldViewOptions,
    zoom: &mut f32,
    pan: &mut Vec2,
) -> Option<u128> {
    let wanted = Vec2::new(ui.available_width(), ui.available_height().max(260.0));
    let (response, mut painter) = ui.allocate_painter(wanted, Sense::click_and_drag());
    let rect = response.rect.shrink(5.0);
    painter.rect_filled(rect, 5.0, Color32::from_rgb(6, 11, 16));

    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll.abs() > 0.0 {
            *zoom = (*zoom * (scroll * 0.0015).exp()).clamp(0.5, 12.0);
        }
    }
    if response.dragged() {
        *pan += response.drag_delta();
    }

    let world_size = Vec2::new(snapshot.width.max(1) as f32, snapshot.height.max(1) as f32);
    let base_scale = (rect.width() / world_size.x)
        .min(rect.height() / world_size.y)
        .max(0.01);
    let scale = base_scale * *zoom;
    let drawn_size = world_size * scale;
    let origin = rect.center() - drawn_size * 0.5 + *pan;
    let world_rect = Rect::from_min_size(origin, drawn_size);
    let clip = rect.intersect(world_rect);
    painter.set_clip_rect(rect);
    painter.rect_filled(world_rect, 2.0, Color32::from_rgb(10, 18, 24));

    draw_grid(&painter, world_rect, snapshot, scale, options.grid_opacity);
    if options.resources {
        for resource in &snapshot.resources {
            let center = tile_center(origin, scale, resource.x, resource.y);
            if clip.contains(center) {
                let radius = (2.0 + (resource.quantity.max(1) as f32).log10()).min(6.0);
                painter.circle_filled(
                    center,
                    radius,
                    resource_color(resource.resource).gamma_multiply(0.62),
                );
            }
        }
    }

    let hover = response.hover_pos().and_then(|pointer| {
        snapshot
            .entities
            .iter()
            .filter_map(|entity| {
                let position = tile_center(origin, scale, entity.x, entity.y);
                let distance = position.distance(pointer);
                (distance <= marker_radius(scale) + 5.0).then_some((distance, entity))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, entity)| entity)
    });

    for entity in &snapshot.entities {
        let center = tile_center(origin, scale, entity.x, entity.y);
        if !clip.expand(10.0).contains(center) {
            continue;
        }
        let is_selected = selected == Some(entity.id);
        let is_hovered = hover.is_some_and(|hovered| hovered.id == entity.id);
        draw_entity(
            &painter,
            entity,
            center,
            scale,
            is_selected,
            is_hovered,
            options,
        );
    }

    if snapshot.entities.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "INITIALIZING WORLD",
            egui::FontId::monospace(13.0),
            muted(),
        );
    }

    if let Some(entity) = hover {
        response.clone().on_hover_ui_at_pointer(|ui| {
            ui.label(egui::RichText::new(format!("Entity #{}", entity.id)).strong());
            ui.label(format!("Money: {}", entity.money));
            ui.label(format!("Action: {}", entity.action_label));
            ui.label(format!("Tile: {}, {}", entity.x, entity.y));
        });
    }

    response
        .clicked()
        .then(|| hover.map(|entity| entity.id))
        .flatten()
}

fn draw_grid(
    painter: &egui::Painter,
    world_rect: Rect,
    snapshot: &SimulationSnapshot,
    scale: f32,
    opacity: f32,
) {
    let interval = (18.0 / scale).ceil().max(1.0) as u32;
    let alpha = (opacity * 255.0) as u8;
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(66, 91, 104, alpha));
    for x in (0..=snapshot.width).step_by(interval as usize) {
        let screen_x = world_rect.left() + x as f32 * scale;
        painter.line_segment(
            [
                Pos2::new(screen_x, world_rect.top()),
                Pos2::new(screen_x, world_rect.bottom()),
            ],
            stroke,
        );
    }
    for y in (0..=snapshot.height).step_by(interval as usize) {
        let screen_y = world_rect.top() + y as f32 * scale;
        painter.line_segment(
            [
                Pos2::new(world_rect.left(), screen_y),
                Pos2::new(world_rect.right(), screen_y),
            ],
            stroke,
        );
    }
    painter.rect_stroke(
        world_rect,
        2.0,
        Stroke::new(1.0, Color32::from_rgb(50, 70, 81)),
        egui::StrokeKind::Inside,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_entity(
    painter: &egui::Painter,
    entity: &EntitySnapshot,
    center: Pos2,
    scale: f32,
    selected: bool,
    hovered: bool,
    options: &WorldViewOptions,
) {
    let radius = marker_radius(scale);
    let color = match entity.action {
        ActionKind::Conserve => Color32::from_rgb(88, 171, 214),
        ActionKind::Predict => Color32::from_rgb(227, 185, 86),
        ActionKind::Give => accent(),
    };
    if selected || hovered {
        painter.circle_stroke(
            center,
            radius + 3.0,
            Stroke::new(if selected { 2.0 } else { 1.0 }, Color32::WHITE),
        );
    }
    painter.circle_filled(center, radius, color);
    if options.goods && !entity.inventory.is_empty() {
        painter.rect_filled(
            Rect::from_center_size(
                center + Vec2::new(radius * 0.75, -radius * 0.75),
                Vec2::splat((radius * 0.8).max(2.5)),
            ),
            1.0,
            Color32::from_rgb(219, 118, 255),
        );
    }
    if options.labels && scale >= 5.0 {
        painter.text(
            center + Vec2::new(radius + 3.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("#{}", entity.id),
            egui::FontId::monospace(9.0),
            Color32::from_gray(205),
        );
    }
}

fn tile_center(origin: Pos2, scale: f32, x: u32, y: u32) -> Pos2 {
    origin + Vec2::new((x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale)
}

fn marker_radius(scale: f32) -> f32 {
    (scale * 0.34).clamp(2.3, 7.0)
}

fn resource_color(resource: usize) -> Color32 {
    const COLORS: [Color32; 5] = [
        Color32::from_rgb(82, 184, 128),
        Color32::from_rgb(73, 150, 198),
        Color32::from_rgb(201, 132, 76),
        Color32::from_rgb(162, 104, 196),
        Color32::from_rgb(194, 178, 79),
    ];
    COLORS[resource % COLORS.len()]
}
