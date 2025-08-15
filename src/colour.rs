#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// True if this colour is a *Reset* command.
    pub reset: bool,

    /// RGB triple — always stored in truecolor space.
    pub rgb: (u8, u8, u8),
}

impl Color {
    /// Reset to terminal defaults.
    pub const fn reset() -> Self {
        Self {
            reset: true,
            rgb: (0, 0, 0),
        }
    }

    /// Create from RGB.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            reset: false,
            rgb: (r, g, b),
        }
    }

    /// Create from an ANSI-256 code.
    pub fn from_ansi256(code: u8) -> Self {
        Self {
            reset: false,
            rgb: Self::ansi256_to_rgb(code),
        }
    }

    /// Convert self to an ANSI-256 index (best match).
    ///
    /// ## return
    /// self converted to an ANSI-256 index, or `None` if it is a reset color.
    pub fn as_ansi256(self) -> Option<u8> {
        if self.reset {
            return None;
        }
        Some(Self::rgb_to_ansi256(self.rgb.0, self.rgb.1, self.rgb.2))
    }

    /// Convert ANSI-256 -> RGB
    pub fn ansi256_to_rgb(code: u8) -> (u8, u8, u8) {
        match code {
            0..=15 => {
                let table = [
                    (0, 0, 0),
                    (128, 0, 0),
                    (0, 128, 0),
                    (128, 128, 0),
                    (0, 0, 128),
                    (128, 0, 128),
                    (0, 128, 128),
                    (192, 192, 192),
                    (128, 128, 128),
                    (255, 0, 0),
                    (0, 255, 0),
                    (255, 255, 0),
                    (0, 0, 255),
                    (255, 0, 255),
                    (0, 255, 255),
                    (255, 255, 255),
                ];
                table[code as usize]
            }
            16..=231 => {
                let c = code - 16;
                let r = c / 36;
                let g = (c % 36) / 6;
                let b = c % 6;
                let to_rgb = |v| if v == 0 { 0 } else { 55 + v * 40 };
                (to_rgb(r), to_rgb(g), to_rgb(b))
            }
            232..=255 => {
                let gray = 8 + (code - 232) * 10;
                (gray, gray, gray)
            }
        }
    }

    // Standard ANSI 16 colors
    pub const Black: Self = Self::rgb(0, 0, 0);
    pub const Maroon: Self = Self::rgb(128, 0, 0);
    pub const Green: Self = Self::rgb(0, 128, 0);
    pub const Olive: Self = Self::rgb(128, 128, 0);
    pub const Navy: Self = Self::rgb(0, 0, 128);
    pub const Purple: Self = Self::rgb(128, 0, 128);
    pub const Teal: Self = Self::rgb(0, 128, 128);
    pub const Silver: Self = Self::rgb(192, 192, 192);

    pub const Grey: Self = Self::rgb(128, 128, 128);
    pub const Red: Self = Self::rgb(255, 0, 0);
    pub const Lime: Self = Self::rgb(0, 255, 0);
    pub const Yellow: Self = Self::rgb(255, 255, 0);
    pub const Blue: Self = Self::rgb(0, 0, 255);
    pub const Fuchsia: Self = Self::rgb(255, 0, 255);
    pub const Aqua: Self = Self::rgb(0, 255, 255);
    pub const White: Self = Self::rgb(255, 255, 255);

    pub const Reset: Self = Self::reset();

    /// Convert RGB → ANSI-256 (nearest).
    pub fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
        let to_6 = |v: u8| -> u8 {
            if v < 48 {
                0
            } else if v < 115 {
                1
            } else {
                ((v - 35) / 40).min(5)
            }
        };
        let r6 = to_6(r);
        let g6 = to_6(g);
        let b6 = to_6(b);

        let cube_index = 16 + 36 * r6 + 6 * g6 + b6;

        let avg = (r as u16 + g as u16 + b as u16) / 3;
        let gray_index = if avg > 238 {
            23
        } else {
            ((avg - 3) / 10).min(23)
        };
        let gray_code = 232 + gray_index as u8;

        let (cr, cg, cb) = Self::ansi256_to_rgb(cube_index);
        let cube_dist = Self::color_dist(r, g, b, cr, cg, cb);
        let (gr, gg, gb) = Self::ansi256_to_rgb(gray_code);
        let gray_dist = Self::color_dist(r, g, b, gr, gg, gb);

        if gray_dist < cube_dist {
            gray_code
        } else {
            cube_index
        }
    }

    fn color_dist(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u32 {
        let dr = r1 as i32 - r2 as i32;
        let dg = g1 as i32 - g2 as i32;
        let db = b1 as i32 - b2 as i32;
        (dr * dr + dg * dg + db * db) as u32
    }
}
