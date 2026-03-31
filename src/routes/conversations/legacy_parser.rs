use crate::routes::conversations::types::{VizKind, VizBlock};

pub fn parse_content(text: &str) -> (Option<String>, Vec<VizBlock>) {
    let mut text_part = None;
    let mut viz_blocks = Vec::new();

    let markers = [
        ("---CHART_SVG---", VizKind::Svg),
        ("---CHART_HTML---", VizKind::Html),
        ("---CHART_ASSET---", VizKind::Asset),
        ("---SEARCH_RESULTS---", VizKind::Search),
    ];

    let mut remaining = text;
    while !remaining.is_empty() {
        let mut found = false;
        for (marker, kind) in &markers {
            if let Some(pos) = remaining.find(marker) {
                if text_part.is_none() && pos > 0 {
                    text_part = Some(remaining[..pos].trim().to_string());
                }
                let after = &remaining[pos + marker.len()..];
                let next_marker_pos = markers.iter()
                    .filter_map(|(m, _)| after.find(m))
                    .min();

                let payload = match next_marker_pos {
                    Some(end) => after[..end].trim().to_string(),
                    None => after.trim().to_string(),
                };

                viz_blocks.push(VizBlock { kind: kind.clone(), payload });
                remaining = match next_marker_pos {
                    Some(end) => &after[end..],
                    None => "",
                };
                found = true;
                break;
            }
        }
        if !found {
            if text_part.is_none() { text_part = Some(remaining.to_string()); }
            break;
        }
    }

    (text_part, viz_blocks)
}
