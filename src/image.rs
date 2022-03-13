use egui::Color32;

pub fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
pub fn default_avatar(name: &str, color: u32) -> egui::ColorImage {
    // TODO: render letters
    egui::ColorImage::new([40, 40], color_from_u32(color))
}

pub fn color_from_u32(v: u32) -> Color32 {
    let b = 0b0000_0000_0000_0000_0000_0000_1111_1111 & v;
    let g = (0b0000_0000_0000_0000_1111_1111_0000_0000 & v) >> 8;
    let r = (0b0000_0000_1111_1111_0000_0000_0000_0000 & v) >> 16;

    Color32::from_rgb(r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_u32() {
        assert_eq!(color_from_u32(0), Color32::from_rgb(0, 0, 0));
        assert_eq!(color_from_u32(0xffffff), Color32::from_rgb(255, 255, 255));
        assert_eq!(color_from_u32(0x0000ff), Color32::from_rgb(0, 0, 255));
        assert_eq!(color_from_u32(0x00ff00), Color32::from_rgb(0, 255, 0));
        assert_eq!(color_from_u32(0xff0000), Color32::from_rgb(255, 0, 0));
        assert_eq!(color_from_u32(0xff8000), Color32::from_rgb(255, 128, 0));
    }
}
