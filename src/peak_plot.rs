use crate::app::MUTED;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};
const COLORS: [Color32; 7] = [
    Color32::from_rgb(113, 83, 190),
    Color32::from_rgb(170, 80, 130),
    Color32::from_rgb(90, 120, 190),
    Color32::from_rgb(180, 115, 40),
    Color32::from_rgb(45, 130, 85),
    Color32::from_rgb(180, 75, 60),
    Color32::from_rgb(125, 130, 40),
];
pub struct Marker {
    pub id: String,
    pub lane: String,
    pub value: f64,
    pub title: String,
    pub enabled: bool,
}
#[allow(clippy::too_many_arguments)]
pub fn plot(
    ui: &mut egui::Ui,
    markers: &[Marker],
    lanes: &[String],
    minimum: f64,
    maximum: f64,
    label: &str,
    selected: Option<&str>,
    accent: Color32,
) -> Option<String> {
    let height = (lanes.len() as f32 * 38. + 70.).max(150.);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(240.), height),
        egui::Sense::click(),
    );
    let painter = ui.painter_at(rect);
    let left = rect.left() + 48.;
    let width = (rect.width() - 65.).max(1.);
    let bottom = rect.bottom() - 36.;
    let x = |v: f64| left + ((v - minimum) / (maximum - minimum)) as f32 * width;
    for tick in 0..=5 {
        let value = minimum + (maximum - minimum) * f64::from(tick) / 5.;
        let pos = x(value);
        painter.line_segment(
            [Pos2::new(pos, rect.top() + 14.), Pos2::new(pos, bottom)],
            Stroke::new(0.5, Color32::from_gray(220)),
        );
        painter.text(
            Pos2::new(pos, bottom + 9.),
            Align2::CENTER_TOP,
            format!("{value:.0}"),
            FontId::proportional(11.),
            MUTED,
        );
    }
    painter.text(
        Pos2::new(rect.center().x, rect.bottom()),
        Align2::CENTER_BOTTOM,
        label,
        FontId::proportional(12.),
        accent,
    );
    if lanes.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Select references above",
            FontId::proportional(14.),
            MUTED,
        );
        return None;
    }
    if markers
        .iter()
        .all(|m| m.value < minimum || m.value > maximum)
    {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No markers in this window",
            FontId::proportional(13.),
            MUTED,
        );
    }
    let mut hovered = Vec::new();
    for (index, lane) in lanes.iter().enumerate() {
        let base = rect.top() + 38. + index as f32 * 38.;
        let color = if lanes.len() == 1 {
            accent
        } else {
            COLORS[index % COLORS.len()]
        };
        painter.text(
            Pos2::new(left - 7., base - 10.),
            Align2::RIGHT_CENTER,
            lane,
            FontId::proportional(11.),
            color,
        );
        painter.line_segment(
            [Pos2::new(left, base), Pos2::new(left + width, base)],
            Stroke::new(0.7, Color32::from_gray(225)),
        );
        for m in markers
            .iter()
            .filter(|m| &m.lane == lane && m.value >= minimum && m.value <= maximum)
        {
            let px = x(m.value);
            let is_selected = Some(m.id.as_str()) == selected;
            let stroke = Stroke::new(
                if is_selected { 3. } else { 1.6 },
                if m.enabled {
                    color
                } else {
                    color.gamma_multiply(0.45)
                },
            );
            let start = Pos2::new(px, base - 24.);
            let end = Pos2::new(px, base);
            if m.enabled {
                painter.line_segment([start, end], stroke);
            } else {
                for offset in [0., 9., 18.] {
                    painter.line_segment(
                        [
                            start + Vec2::new(0., offset),
                            start + Vec2::new(0., (offset + 5.).min(24.)),
                        ],
                        stroke,
                    );
                }
            }
            if is_selected {
                painter.circle_filled(start, 3., color);
            }
            if let Some(p) = response.hover_pos() {
                let distance = (p.x - px).abs();
                if distance <= 7. && p.y >= base - 30. && p.y <= base + 6. {
                    hovered.push((m, distance));
                }
            }
        }
    }
    hovered.sort_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((m, _)) = hovered.first() {
        let clicked = response.clicked();
        let id = m.id.clone();
        response.on_hover_text(
            hovered
                .iter()
                .map(|(m, _)| m.title.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        if clicked {
            return Some(id);
        }
    }
    None
}
