use anyhow::Result;

fn encode_webp(img: &image::DynamicImage, quality: f32) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let encoded = zenwebp::encoder::Encoder::new_rgba(&rgba, w, h)
        .quality(quality)
        .encode()?;
    Ok(encoded)
}

pub struct ImageOptimizer;

impl ImageOptimizer {
    pub fn to_webp(raw_bytes: &[u8], quality: f32) -> Result<Vec<u8>> {
        let img = image::load_from_memory(raw_bytes)?;
        encode_webp(&img, quality)
    }

    pub fn for_llm_vision(raw_bytes: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(raw_bytes)?;
        let img = if img.width() > 2048 || img.height() > 2048 {
            img.resize(2048, 2048, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };
        encode_webp(&img, 75.0)
    }
}
