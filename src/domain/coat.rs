use serde::{Deserialize, Serialize};

/// A dragon's scale colour. The art is drawn lavender; any other coat is made by shifting just the
/// scale and wing colours to a new hue as frames load, so the golden belly, cream horns, amber eyes,
/// pink cheeks, the outline and the fire keep their own colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coat {
    #[default]
    Natural,
    Red,
    Pink,
    Blue,
    Green,
    Gold,
    Shadow,
}

/// The hue (degrees) the lavender art is centred on, and the band either side of it that counts
/// as "scales": wide enough for the shading and the paler wings, narrow enough to miss everything else.
const ART_HUE: f32 = 257.0;
const BAND: f32 = 28.0;

impl Coat {
    pub const ALL: [Coat; 7] = [Coat::Natural, Coat::Red, Coat::Pink, Coat::Blue, Coat::Green, Coat::Gold, Coat::Shadow];

    pub fn title(self) -> &'static str {
        match self {
            Coat::Natural => "💜 Lavender (natural)",
            Coat::Red => "❤️ Red",
            Coat::Pink => "🩷 Pink",
            Coat::Blue => "💙 Blue",
            Coat::Green => "💚 Green",
            Coat::Gold => "💛 Gold",
            Coat::Shadow => "🖤 Shadow",
        }
    }

    /// Where the scales go: (hue, saturation scale, brightness scale); None keeps the art as drawn.
    fn target(self) -> Option<(f32, f32, f32)> {
        match self {
            Coat::Natural => None,
            Coat::Red => Some((2.0, 1.25, 0.95)),
            Coat::Pink => Some((328.0, 0.95, 1.08)),
            Coat::Blue => Some((208.0, 1.1, 1.0)),
            Coat::Green => Some((128.0, 0.95, 0.92)),
            Coat::Gold => Some((42.0, 1.25, 1.08)),
            Coat::Shadow => Some((262.0, 0.55, 0.42)),
        }
    }

    /// Recolours one straight (not premultiplied) RGB pixel; anything outside the scale colours is
    /// returned as it is.
    pub fn recolor(self, rgb: [u8; 3]) -> [u8; 3] {
        let Some((hue, sat_k, val_k)) = self.target() else { return rgb };
        let (h, s, v) = to_hsv(rgb);
        let off = ((h - ART_HUE + 540.0) % 360.0) - 180.0;
        // Very dark (the outline) and washed-out pixels aren't scales.
        if off.abs() > BAND || s < 0.15 || v < 0.3 { return rgb }
        from_hsv((hue + off + 360.0) % 360.0, (s * sat_k).min(1.0), (v * val_k).min(1.0))
    }
}

fn to_hsv([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let d = max - r.min(g).min(b);
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    ((h + 360.0) % 360.0, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [((r + m) * 255.0).round() as u8, ((g + m) * 255.0).round() as u8, ((b + m) * 255.0).round() as u8]
}

#[cfg(test)]
#[path = "coat_tests.rs"]
mod tests;
