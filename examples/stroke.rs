#![allow(unused)]

use ink_stroke_modeler_rs::{ModelerInput, ModelerInputEventType, ModelerResult, StrokeModeler};
use svg::Node;

fn main() -> anyhow::Result<()> {
    let bounds = AABB {
        mins: (0.0, 0.0),
        maxs: (6.0, 4.0),
    };

    let stroke = vec![
        ModelerInput::new(ModelerInputEventType::kDown, (1.0, 1.0), 0.0, 0.1, 0.0, 0.0),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (1.2, 2.0),
            0.02,
            0.3,
            0.0,
            0.0,
        ),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (1.4, 2.3),
            0.04,
            0.5,
            0.0,
            0.0,
        ),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (3.5, 2.0),
            0.06,
            0.8,
            0.0,
            0.0,
        ),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (4.0, 2.5),
            0.12,
            0.9,
            0.0,
            0.0,
        ),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (5.0, 3.0),
            0.13,
            0.8,
            0.0,
            0.0,
        ),
        ModelerInput::new(
            ModelerInputEventType::kMove,
            (4.8, 3.1),
            0.13,
            0.7,
            0.0,
            0.0,
        ),
        ModelerInput::new(ModelerInputEventType::kUp, (4.5, 3.0), 0.14, 0.2, 0.0, 0.0),
    ];
    let input_elements = stroke
        .iter()
        .map(Element::from_modeler_input)
        .collect::<Vec<Element>>();
    Element::create_svg(
        &input_elements,
        bounds,
        std::path::PathBuf::from("./examples/stroke/input.svg"),
    )?;

    let mut modeler = StrokeModeler::default();

    let results = stroke
        .into_iter()
        .flat_map(|i| modeler.update(i))
        .collect::<Vec<ModelerResult>>();

    let result_elements = results
        .iter()
        .map(Element::from_modeler_result)
        .collect::<Vec<Element>>();
    Element::create_svg(
        &result_elements,
        bounds,
        std::path::PathBuf::from("./examples/stroke/results.svg"),
    )?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Element {
    pos: (f32, f32),
    vel: Option<(f32, f32)>,
    time: f64,
    pressure: f32,
    tilt: f32,
    orientation: f32,
}

impl Element {
    fn from_modeler_input(i: &ModelerInput) -> Self {
        Self {
            pos: i.get_pos(),
            vel: None,
            time: i.get_time(),
            pressure: i.get_pressure(),
            tilt: i.get_tilt(),
            orientation: i.get_orientation(),
        }
    }

    fn from_modeler_result(r: &ModelerResult) -> Self {
        Self {
            pos: r.get_pos(),
            vel: Some(r.get_velocity()),
            time: r.get_time(),
            pressure: r.get_pressure(),
            tilt: r.get_tilt(),
            orientation: r.get_orientation(),
        }
    }

    fn create_svg(
        elements: &[Self],
        bounds: AABB,
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
            doc.append(
                svg::node::element::Line::new()
                    .set("x1", start.pos.0)
                    .set("y1", start.pos.1)
                    .set("x2", end.pos.0)
                    .set("y2", end.pos.1)
                    //.set("stroke-width", (end.pressure + start.pressure) / 2.0)
                    .set("stroke-width", 0.01)
                    .set("stroke", "black"),
            );
        }

        Ok(svg::save(file, &doc)?)
    }
}

#[derive(Debug, Clone, Copy)]
struct AABB {
    mins: (f32, f32),
    maxs: (f32, f32),
}

impl AABB {
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
