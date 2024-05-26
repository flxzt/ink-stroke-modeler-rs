#![allow(unused)]

use ink_stroke_modeler_rs::{
    ModelerInput, ModelerInputEventType, ModelerParams, ModelerResult, StrokeModeler,
};
use svg::Node;

fn main() -> anyhow::Result<()> {
    let bounds = Aabb {
        mins: (0.0, 0.0),
        maxs: (300.0, 300.0),
    };

    let input_stroke = vec![
        ModelerInput {
            event_type: ModelerInputEventType::kDown,
            pos: (90.0, 30.0),
            time: 0.0,
            pressure: 0.25,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (30.0, 45.0),
            time: 0.02,
            pressure: 0.3,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (60.0, 240.0),
            time: 0.04,
            pressure: 0.7,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (105.0, 270.0),
            time: 0.06,
            pressure: 1.0,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (180.0, 30.0),
            time: 0.12,
            pressure: 0.3,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (120.0, 150.0),
            time: 0.10,
            pressure: 0.6,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (240.0, 120.0),
            time: 0.16,
            pressure: 0.3,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (150.0, 210.0),
            time: 0.20,
            pressure: 0.8,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (210.0, 240.0),
            time: 0.22,
            pressure: 0.8,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kMove,
            pos: (255.0, 240.0),
            pressure: 0.24,
            time: 0.7,
        },
        ModelerInput {
            event_type: ModelerInputEventType::kUp,
            pos: (270.0, 270.0),
            pressure: 0.26,
            time: 0.5,
        },
    ];
    let input_elements = input_stroke
        .iter()
        .map(Element::from_modeler_input)
        .collect::<Vec<Element>>();
    create_svg(
        &input_elements,
        bounds,
        std::path::PathBuf::from("./examples/stroke/input.svg"),
    )?;

    let mut modeler = StrokeModeler::default();

    let result_stroke = input_stroke
        .into_iter()
        .filter_map(|i| {
            modeler
                .update(i)
                .map_err(|e| eprintln!("modeler updated, Err: {e:?}"))
                .ok()
        })
        .flatten()
        .collect::<Vec<ModelerResult>>();
    let result_elements = result_stroke
        .iter()
        .map(Element::from_modeler_result)
        .collect::<Vec<Element>>();
    create_svg(
        &result_elements,
        bounds,
        std::path::PathBuf::from("./examples/stroke/modeled.svg"),
    )?;

    Ok(())
}

#[derive(Debug, Clone)]
struct Element {
    pos: (f32, f32),
    velocity: Option<(f32, f32)>,
    time: f64,
    pressure: f32,
}

impl Element {
    fn from_modeler_input(i: &ModelerInput) -> Self {
        Self {
            pos: i.pos,
            velocity: None,
            time: i.time,
            pressure: i.pressure,
        }
    }

    fn from_modeler_result(r: &ModelerResult) -> Self {
        Self {
            pos: r.pos,
            velocity: Some(r.velocity),
            time: r.time,
            pressure: r.pressure,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Aabb {
    mins: (f32, f32),
    maxs: (f32, f32),
}

impl Aabb {
    fn new_invalid() -> Self {
        Self {
            mins: (f32::MAX, f32::MAX),
            maxs: (f32::MIN, f32::MIN),
        }
    }

    fn width(&self) -> f32 {
        self.maxs.0 - self.mins.0
    }
    fn height(&self) -> f32 {
        self.maxs.1 - self.mins.1
    }

    fn extend(&mut self, coord: (f32, f32)) {
        self.mins.0 = self.mins.0.min(coord.0);
        self.mins.1 = self.mins.1.min(coord.1);
        self.maxs.0 = self.maxs.0.max(coord.0);
        self.maxs.1 = self.maxs.1.max(coord.1);
    }
}

fn create_svg(
    elements: &[Element],
    bounds: Aabb,
    file: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    let mut doc = svg::Document::new()
        .set("x", bounds.mins.0)
        .set("y", bounds.mins.1)
        .set("width", bounds.width())
        .set("height", bounds.height());

    doc.append(
        svg::node::element::Rectangle::new()
            .set("x", bounds.mins.0)
            .set("y", bounds.mins.1)
            .set("width", bounds.width())
            .set("height", bounds.height())
            .set("fill", "white"),
    );

    for (start, end) in elements.iter().zip(elements.iter().skip(1)) {
        let brightness = 1.0 / (end.pressure + start.pressure) / 2.0;
        doc.append(
            svg::node::element::Line::new()
                .set("x1", start.pos.0)
                .set("y1", start.pos.1)
                .set("x2", end.pos.0)
                .set("y2", end.pos.1)
                .set(
                    "stroke",
                    format!("hsl(200, 100%, {}%", (brightness * 100.0).round()),
                )
                .set("stroke-width", 2.0)
                .set("stroke-linecap", "round"),
        );
    }

    Ok(svg::save(file, &doc)?)
}
