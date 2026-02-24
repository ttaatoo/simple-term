// Convenience method to convert from a GPUI color to an alacritty Rgb
use alacritty_terminal::vte::ansi::Rgb as AlacRgb;
use gpui::Rgba;

pub fn to_alac_rgb(color: impl Into<Rgba>) -> AlacRgb {
    let color = color.into();
    let r = ((color.r * color.a) * 255.) as u8;
    let g = ((color.g * color.a) * 255.) as u8;
    let b = ((color.b * color.a) * 255.) as u8;
    AlacRgb { r, g, b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_opaque_color_directly() {
        let rgb = to_alac_rgb(Rgba {
            r: 1.0,
            g: 0.5,
            b: 0.0,
            a: 1.0,
        });
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 127);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn premultiplies_alpha() {
        let rgb = to_alac_rgb(Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.5,
        });
        assert_eq!(rgb.r, 127);
        assert_eq!(rgb.g, 127);
        assert_eq!(rgb.b, 127);
    }
}
