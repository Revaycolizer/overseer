use plotters::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChartRequest {
    pub chart_type: String,
    pub title: String,
    pub labels: Vec<String>,
    pub data: Vec<f64>,
}

/// Render SVG directly, without embedding a JavaScript runtime in the backend.
pub fn generate_chart_svg(req: ChartRequest) -> Result<String, String> {
    if req.labels.len() != req.data.len() {
        return Err(format!(
            "Labels count ({}) does not match data count ({})",
            req.labels.len(),
            req.data.len()
        ));
    }
    if req.data.is_empty() || req.data.len() > 1000 {
        return Err("Charts require between 1 and 1000 data points".into());
    }
    if req
        .data
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1e100)
    {
        return Err("Chart values must be finite and have magnitude at most 1e100".into());
    }
    let kind = req.chart_type.to_lowercase();
    if !["bar", "line", "pie"].contains(&kind.as_str()) {
        return Err(format!("Unsupported chart type: {}", req.chart_type));
    }
    if kind == "pie"
        && (req.data.len() < 2
            || req.data.iter().any(|value| *value < 0.0)
            || req.data.iter().sum::<f64>() <= 0.0)
    {
        return Err(
            "Pie charts require at least two nonnegative values with a positive total".into(),
        );
    }
    let mut svg = String::new();
    {
        let root = SVGBackend::with_string(&mut svg, (1200, 600)).into_drawing_area();
        root.fill(&WHITE).map_err(|error| error.to_string())?;
        let color = RGBColor(59, 130, 246);
        if kind == "pie" {
            root.draw(&Text::new(
                req.title.as_str(),
                (600, 30),
                ("sans-serif", 24).into_font().color(&BLACK).pos(
                    plotters::style::text_anchor::Pos::new(
                        plotters::style::text_anchor::HPos::Center,
                        plotters::style::text_anchor::VPos::Top,
                    ),
                ),
            ))
            .map_err(|error| error.to_string())?;
            let center = (600, 315);
            let radius = 210.0;
            let colors: Vec<RGBColor> = (0..req.data.len())
                .map(|index| {
                    let (r, g, b) = Palette99::pick(index).rgb();
                    RGBColor(r, g, b)
                })
                .collect();
            let labels: Vec<String> = req
                .labels
                .iter()
                .zip(&req.data)
                .map(|(label, value)| format!("{label}: {value}"))
                .collect();
            let mut pie = Pie::new(&center, &radius, &req.data, &colors, &labels);
            pie.label_style(("sans-serif", 14).into_font());
            pie.percentages(("sans-serif", 14).into_font());
            root.draw(&pie).map_err(|error| error.to_string())?;
        } else {
            let min = req.data.iter().copied().fold(0.0, f64::min);
            let max = req.data.iter().copied().fold(0.0, f64::max);
            let padding = ((max - min) * 0.1).max(1.0);
            let low = if min < 0.0 { min - padding } else { 0.0 };
            let high = max + padding;
            let mut chart = ChartBuilder::on(&root)
                .caption(&req.title, ("sans-serif", 24))
                .margin(25)
                .x_label_area_size(70)
                .y_label_area_size(75)
                .build_cartesian_2d(0.0..req.data.len() as f64, low..high)
                .map_err(|error| error.to_string())?;
            chart
                .configure_mesh()
                .disable_x_mesh()
                .x_labels(req.labels.len().min(12))
                .x_label_formatter(&|x| {
                    let index = x.floor() as usize;
                    req.labels.get(index).cloned().unwrap_or_default()
                })
                .draw()
                .map_err(|error| error.to_string())?;
            if kind == "bar" {
                chart
                    .draw_series(req.data.iter().enumerate().map(|(index, value)| {
                        Rectangle::new(
                            [(index as f64 + 0.15, 0.0), (index as f64 + 0.85, *value)],
                            color.filled(),
                        )
                    }))
                    .map_err(|error| error.to_string())?;
            } else {
                chart
                    .draw_series(LineSeries::new(
                        req.data
                            .iter()
                            .enumerate()
                            .map(|(index, value)| (index as f64 + 0.5, *value)),
                        color.stroke_width(3),
                    ))
                    .map_err(|error| error.to_string())?;
                chart
                    .draw_series(req.data.iter().enumerate().map(|(index, value)| {
                        Circle::new((index as f64 + 0.5, *value), 4, color.filled())
                    }))
                    .map_err(|error| error.to_string())?;
            }
            chart
                .draw_series(req.data.iter().enumerate().map(|(index, value)| {
                    Text::new(
                        value.to_string(),
                        (index as f64 + 0.5, *value),
                        ("sans-serif", 12).into_font(),
                    )
                }))
                .map_err(|error| error.to_string())?;
        }
        root.present().map_err(|error| error.to_string())?;
    }
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: &str, values: Vec<f64>) -> ChartRequest {
        ChartRequest {
            chart_type: kind.into(),
            title: "Sales < & >".into(),
            labels: values
                .iter()
                .enumerate()
                .map(|(index, _)| format!("Item {index}"))
                .collect(),
            data: values,
        }
    }

    #[test]
    fn renders_all_supported_charts_and_escapes_text() {
        for kind in ["bar", "line", "pie"] {
            let svg = generate_chart_svg(request(kind, vec![10.0, 20.0, 15.0])).unwrap();
            assert!(svg.contains("<svg"));
            assert!(svg.contains("Sales &lt; &amp; &gt;"));
            assert!(svg.contains("Item 0"));
            assert!(!svg.contains("NaN"));
        }
    }

    #[test]
    fn handles_zero_and_negative_cartesian_values() {
        for values in [vec![0.0, 0.0], vec![-10.0, 5.0]] {
            for kind in ["bar", "line"] {
                assert!(generate_chart_svg(request(kind, values.clone())).is_ok());
            }
        }
    }

    #[test]
    fn rejects_invalid_data() {
        for (kind, values) in [
            ("pie", vec![0.0, 0.0]),
            ("pie", vec![-1.0, 2.0]),
            ("pie", vec![1.0]),
            ("bar", vec![]),
            ("line", vec![f64::NAN]),
            ("bar", vec![f64::INFINITY]),
        ] {
            assert!(generate_chart_svg(request(kind, values)).is_err());
        }
        let mut req = request("bar", vec![1.0]);
        req.labels.clear();
        assert!(generate_chart_svg(req).is_err());
    }
}
