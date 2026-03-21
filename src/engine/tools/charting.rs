use charming::{
    component::{Axis, Legend, Title, Grid},
    element::{AxisType, Tooltip, Trigger, AreaStyle, ItemStyle, Label, LineStyle},
    series::{Bar, Line, Pie},
    Chart,
    ImageRenderer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChartRequest {
    pub chart_type: String, // "bar", "line", "pie"
    pub title: String,
    pub labels: Vec<String>,
    pub data: Vec<f64>,
}

pub fn generate_chart_svg(req: ChartRequest) -> Result<String, String> {
    // Convert f64 data to charming's expected format

    if req.labels.len() != req.data.len() {
        return Err(format!(
            "Labels count ({}) does not match data count ({})",
            req.labels.len(), req.data.len()
        ));
    }
    
    // Reject single data point for pie charts
    if req.chart_type == "pie" && req.data.len() < 2 {
        return Err("Pie chart requires at least 2 data points".to_string());
    }
    let data_values: Vec<charming::datatype::DataPoint> = req.data
        .iter()
        .map(|&v| charming::datatype::DataPoint::Value(charming::datatype::CompositeValue::from(v)))
        .collect();

    let svg = match req.chart_type.to_lowercase().as_str() {
        "bar" => render_bar(&req.title, &req.labels, &req.data),
        "line" => render_line(&req.title, &req.labels, &req.data),
        "pie" => render_pie(&req.title, &req.labels, &req.data),
        _ => return Err(format!("Unsupported chart type: {}", req.chart_type)),
    }?;

    Ok(format!("---CHART_SVG---\n{}", svg))
}

fn make_renderer() -> ImageRenderer {
    ImageRenderer::new(1200, 600)
}

fn render_bar(title: &str, labels: &[String], data: &[f64]) -> Result<String, String> {
    let chart = Chart::new()
        .background_color("#ffffff")
        .title(
            Title::new()
                .text(title)
                .left("center")
                .top("10")
        )
        .grid(
            Grid::new()
                .left("10%")
                .right("5%")
                .top("15%")
                .bottom("20%")
                .contain_label(true)
        )
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(labels.to_vec())
                .axis_label(
                    charming::element::AxisLabel::new()
                        .interval(0)
                        .rotate(0.0)
                        .font_size(12.0)
                )
        )
        .y_axis(
            Axis::new()
                .type_(AxisType::Value)
                .min(0.0)
                .axis_label(
                    charming::element::AxisLabel::new()
                        .show(true)
                        .font_size(12.0)
                )
        )
        .series(
            Bar::new()
                .name("Value")
                .data(data.iter().map(|&v| v).collect::<Vec<f64>>())
                .item_style(
                    ItemStyle::new()
                        .color("#3b82f6")
                        .border_radius(4.0_f64)
                )
                .label(Label::new().show(true))
        );

    make_renderer()
        .render(&chart)
        .map_err(|e| format!("Bar render error: {e:?}"))
}

fn render_line(title: &str, labels: &[String], data: &[f64]) -> Result<String, String> {
   let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;
    
    // Always start from 0 if min is close to 0, otherwise add padding
    let y_min = 0.0_f64;  // always start from 0
    let y_max = (max_val * 1.3).ceil();

    let chart = Chart::new()
        .background_color("#ffffff")
        .title(Title::new().text(title).left("center").top("10"))
       .grid(
            Grid::new()
                .left("12%").right("8%").top("20%").bottom("15%")
                .contain_label(true)
        )
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(labels.to_vec())
                .boundary_gap(false)
                .axis_label(
                    charming::element::AxisLabel::new()
                        .show(true)
                        .interval(0)
                )
        )
        .y_axis(
            Axis::new()
                .type_(AxisType::Value)
                .min(y_min)
                .max(y_max)
        )
        .series(
            Line::new()
                .name("Value")
                .data(data.iter().map(|&v| v).collect::<Vec<f64>>())
                .smooth(false)
                .symbol_size(10)
                .item_style(ItemStyle::new().color("#3b82f6"))
                .line_style(LineStyle::new().width(3).color("#3b82f6"))
                .area_style(AreaStyle::new().opacity(0.15))
                .label(Label::new().show(true))
        );

    make_renderer()
        .render(&chart)
        .map_err(|e| format!("Line render error: {e:?}"))
}

fn render_pie(title: &str, labels: &[String], data: &[f64]) -> Result<String, String> {
    let pie_data: Vec<(f64, String)> = data
        .iter()
        .zip(labels.iter())
        .map(|(&v, l)| (v, l.clone()))
        .collect();

    let chart = Chart::new()
        .background_color("#ffffff")
        .title(
            Title::new()
                .text(title)
                .left("center")
                .top("16")
        )
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(
            Legend::new()
                .bottom("0")
                .left("center")
        )
        .series(
            Pie::new()
                .name("Value")
                .data(pie_data)
                .radius(vec!["35%", "65%"])
                .center(vec!["50%", "50%"])
                // .label(
                //     Label::new()
                //         .show(true)
                //         .formatter("{b}: {d}%")
                // )
                .item_style(ItemStyle::new().border_radius(6.0_f64))
        );

    make_renderer()
        .render(&chart)
        .map_err(|e| format!("Pie render error: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_chart() {
        let req = ChartRequest {
            chart_type: "bar".to_string(),
            title: "Fruit Sales".to_string(),
            labels: vec!["Apples".to_string(), "Bananas".to_string(), "Oranges".to_string()],
            data: vec![10.0, 20.0, 15.0],
        };
        let result = generate_chart_svg(req).unwrap();
        assert!(result.starts_with("---CHART_SVG---"));
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_line_chart() {
        let req = ChartRequest {
            chart_type: "line".to_string(),
            title: "Next.js Growth".to_string(),
            labels: vec!["2024".to_string(), "2025".to_string(), "2026".to_string()],
            data: vec![1.8, 2.7, 3.0],
        };
        let result = generate_chart_svg(req).unwrap();
        assert!(result.starts_with("---CHART_SVG---"));
        assert!(result.contains("<svg"));
    }

    #[test]
    fn test_pie_chart() {
        let req = ChartRequest {
            chart_type: "pie".to_string(),
            title: "Market Share".to_string(),
            labels: vec!["Next.js".to_string(), "Nuxt".to_string(), "Other".to_string()],
            data: vec![45.0, 30.0, 25.0],
        };
        let result = generate_chart_svg(req).unwrap();
        assert!(result.starts_with("---CHART_SVG---"));
        assert!(result.contains("<svg"));
    }
}