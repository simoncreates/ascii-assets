use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
};

pub mod colour;
pub use colour::Color;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
/// A single character together with optional foreground / background colours
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TerminalChar {
    pub chr: char,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl From<char> for TerminalChar {
    fn from(chr: char) -> Self {
        TerminalChar::from_char(chr)
    }
}

impl From<(char, Color)> for TerminalChar {
    fn from((chr, fg): (char, Color)) -> Self {
        TerminalChar::with_fg(chr, fg)
    }
}

impl TerminalChar {
    fn default() -> Self {
        Self {
            chr: ' ',
            fg_color: None,
            bg_color: None,
        }
    }

    pub fn from_char<C: Into<char>>(chr: C) -> Self {
        Self {
            chr: chr.into(),
            fg_color: None,
            bg_color: None,
        }
    }

    pub fn set_fg(mut self, fg: Color) -> Self {
        self.fg_color = Some(fg);
        self
    }

    pub fn set_bg(mut self, bg: Color) -> Self {
        self.bg_color = Some(bg);
        self
    }

    /// Create a TerminalChar with foreground color.
    pub fn with_fg<C: Into<char>>(chr: C, fg: Color) -> Self {
        Self {
            chr: chr.into(),
            fg_color: Some(fg),
            bg_color: None,
        }
    }

    /// Create a TerminalChar with background color.
    pub fn with_bg<C: Into<char>>(chr: C, bg: Color) -> Self {
        Self {
            chr: chr.into(),
            fg_color: None,
            bg_color: Some(bg),
        }
    }

    /// Create a TerminalChar with both foreground and background colors.
    pub fn with_colors<C: Into<char>>(chr: C, fg: Color, bg: Color) -> Self {
        Self {
            chr: chr.into(),
            fg_color: Some(fg),
            bg_color: Some(bg),
        }
    }

    /// Convert the foreground colour to an ANSI-256 index if possible.
    pub fn fg_to_ansi256(&self) -> Option<u8> {
        self.fg_color.and_then(|c| c.as_ansi256())
    }

    /// Convert the background colour to an ANSI-256 index if possible.
    pub fn bg_to_ansi256(&self) -> Option<u8> {
        self.bg_color.and_then(|c| c.as_ansi256())
    }

    /// Write a character to the writer
    ///   u32 little-endian code point
    ///   u8 flag + 3×u8 for optional foreground RGB
    ///   u8 flag + 3×u8 for optional background RGB
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.chr as u32)?;

        // Foreground colour
        if let Some(col) = self.fg_color {
            if !col.reset {
                w.write_u8(1)?;
                let (r, g, b) = col.rgb;
                w.write_u8(r)?;
                w.write_u8(g)?;
                w.write_u8(b)?;
            } else {
                w.write_u8(0)?;
            }
        } else {
            w.write_u8(0)?;
        }

        // Background colour
        if let Some(col) = self.bg_color {
            if !col.reset {
                w.write_u8(1)?;
                let (r, g, b) = col.rgb;
                w.write_u8(r)?;
                w.write_u8(g)?;
                w.write_u8(b)?;
            } else {
                w.write_u8(0)?;
            }
        } else {
            w.write_u8(0)?;
        }

        Ok(())
    }

    /// Read a character from the same binary format.
    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let code = r.read_u32::<LittleEndian>()?;
        let chr = std::char::from_u32(code).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid Unicode scalar value")
        })?;

        // Foreground colour
        let fg_color = if r.read_u8()? == 1 {
            let r8 = r.read_u8()?;
            let g8 = r.read_u8()?;
            let b8 = r.read_u8()?;
            Some(Color::rgb(r8, g8, b8))
        } else {
            None
        };

        // Background colour
        let bg_color = if r.read_u8()? == 1 {
            let r8 = r.read_u8()?;
            let g8 = r.read_u8()?;
            let b8 = r.read_u8()?;
            Some(Color::rgb(r8, g8, b8))
        } else {
            None
        };

        Ok(Self {
            chr,
            fg_color,
            bg_color,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalString(pub Vec<TerminalChar>);

impl FromIterator<TerminalChar> for TerminalString {
    fn from_iter<I: IntoIterator<Item = TerminalChar>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for TerminalString {
    type Item = TerminalChar;
    type IntoIter = std::vec::IntoIter<TerminalChar>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a TerminalString {
    type Item = &'a TerminalChar;
    type IntoIter = std::slice::Iter<'a, TerminalChar>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut TerminalString {
    type Item = &'a mut TerminalChar;
    type IntoIter = std::slice::IterMut<'a, TerminalChar>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

// Convenience: create TerminalString from a &str, all default colors
impl From<&str> for TerminalString {
    fn from(s: &str) -> Self {
        s.chars().map(TerminalChar::from).collect()
    }
}

/// A single frame (sprite) of ASCII art.
#[derive(Debug, PartialEq, Clone)]
pub struct AsciiSprite {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<TerminalChar>,
}

impl AsciiSprite {
    /// Create a sprite,
    ///
    /// ## Error
    /// if `width * height` doesn't match with the size of the pixel-vector
    pub fn new(width: u16, height: u16, pixels: Vec<TerminalChar>) -> io::Result<Self> {
        if pixels.len() != (width as usize) * (height as usize) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "pixel count {} does not match width*height ({})",
                    pixels.len(),
                    (width as usize) * (height as usize)
                ),
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Serialise the sprite
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        for p in &self.pixels {
            p.write_to(w)?;
        }
        Ok(())
    }

    /// Deserialise a sprite given its dimensions
    pub fn read_from<R: Read>(r: &mut R, width: u16, height: u16) -> io::Result<Self> {
        let mut pixels = Vec::with_capacity((width as usize) * (height as usize));
        for _ in 0..(width as usize * height as usize) {
            pixels.push(TerminalChar::read_from(r)?);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Return the sprites pixel buffer as a two-dimensional grid.
    pub fn as_grid(&self) -> Vec<Vec<TerminalChar>> {
        let mut grid = Vec::with_capacity(self.height as usize);
        for row in 0..self.height {
            let mut rvec = Vec::with_capacity(self.width as usize);
            for col in 0..self.width {
                let idx = (row as usize) * self.width as usize + col as usize;
                rvec.push(self.pixels[idx]);
            }
            grid.push(rvec);
        }
        grid
    }
    /// Return the sprites Pixel buffer as a flat vector.
    pub fn as_flat(&self) -> Vec<TerminalChar> {
        self.pixels.clone()
    }

    /// Get a character at the given coordinates, or ``None`` if out of bounds
    pub fn get_char(&self, x: u16, y: u16) -> Option<TerminalChar> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize) * self.width as usize + x as usize;
        Some(self.pixels[idx])
    }
}

/// A collection of frames that share the same dimensions.
#[derive(Debug, PartialEq, Clone)]
pub struct AsciiVideo {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<AsciiSprite>,
}

impl AsciiVideo {
    const MAGIC: [u8; 4] = *b"ASCV";
    const VERSION: u8 = 1;

    /// Create a new video
    pub fn new(width: u16, height: u16, frames: Vec<AsciiSprite>) -> io::Result<Self> {
        for (i, f) in frames.iter().enumerate() {
            if f.width != width || f.height != height {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Frame {} has size {}x{} but expected {}x{}",
                        i, f.width, f.height, width, height
                    ),
                ));
            }
        }
        Ok(Self {
            width,
            height,
            frames,
        })
    }

    /// Return the number of frames and the dimensions.
    /// (frame_count, height, width)
    pub fn size(&self) -> (usize, usize, usize) {
        (self.frames.len(), self.height as usize, self.width as usize)
    }

    pub fn write_to_file(&self, path: &str) -> io::Result<()> {
        let f = File::create(path)?;
        let mut w = BufWriter::new(f);

        // Header
        w.write_all(&Self::MAGIC)?;
        w.write_u8(Self::VERSION)?;
        w.write_u16::<LittleEndian>(self.width)?;
        w.write_u16::<LittleEndian>(self.height)?;
        w.write_u32::<LittleEndian>(self.frames.len() as u32)?;

        // Frames
        for f in &self.frames {
            f.write_to(&mut w)?;
        }

        w.flush()
    }

    pub fn read_from_file(path: &str) -> io::Result<Self> {
        let f = File::open(path)?;
        let mut r = BufReader::new(f);

        // Header
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if magic != Self::MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bad magic number",
            ));
        }

        let ver = r.read_u8()?;
        if ver != Self::VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported version {}", ver),
            ));
        }

        let width = r.read_u16::<LittleEndian>()?;
        let height = r.read_u16::<LittleEndian>()?;
        let frame_count = r.read_u32::<LittleEndian>()? as usize;

        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "dimensions out of range, max 4096x4096",
            ));
        }

        if frame_count > 100_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("too many frames: {} (max {})", frame_count, 100_000),
            ));
        }

        // frames
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            frames.push(AsciiSprite::read_from(&mut r, width, height)?);
        }

        Self::new(width, height, frames)
    }

    /// Return a single frame as a two-dimensional grid.
    pub fn get_frame(&self, index: usize) -> Option<Vec<Vec<TerminalChar>>> {
        self.frames.get(index).map(|s| s.as_grid())
    }

    /// Return a single frame as a flat vector.
    pub fn get_frame_flat(&self, index: usize) -> Option<Vec<TerminalChar>> {
        Some(self.frames.get(index)?.as_flat())
    }

    /// Convert all frames to grids.    
    ///
    /// ### Warning
    /// Use only when you really need a two-dimensional representation
    /// the operation is O(n^2)
    pub fn frames_as_grid(&self) -> Vec<Vec<Vec<TerminalChar>>> {
        self.frames.iter().map(|s| s.as_grid()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_video_size() {
        let pixels = vec![
            TerminalChar {
                chr: 'x',
                fg_color: None,
                bg_color: None
            };
            6
        ];
        let sprite1 = AsciiSprite::new(2, 3, pixels.clone()).unwrap();
        let sprite2 = AsciiSprite::new(2, 3, pixels).unwrap();

        let video = AsciiVideo::new(2, 3, vec![sprite1, sprite2]).unwrap();
        assert_eq!(video.size(), (2, 3, 2));
    }

    #[test]
    fn test_sprite_grid_access() {
        let pixels = vec![
            TerminalChar {
                chr: 'a',
                fg_color: None,
                bg_color: None,
            },
            TerminalChar {
                chr: 'b',
                fg_color: None,
                bg_color: None,
            },
            TerminalChar {
                chr: 'c',
                fg_color: None,
                bg_color: None,
            },
            TerminalChar {
                chr: 'd',
                fg_color: None,
                bg_color: None,
            },
        ];
        let sprite = AsciiSprite::new(2, 2, pixels).unwrap();

        let grid = sprite.as_grid();
        assert_eq!(grid[0][0].chr, 'a');
        assert_eq!(grid[0][1].chr, 'b');
        assert_eq!(grid[1][0].chr, 'c');
        assert_eq!(grid[1][1].chr, 'd');

        assert_eq!(sprite.get_char(0, 0).unwrap().chr, 'a');
        assert_eq!(sprite.get_char(1, 0).unwrap().chr, 'b');
        assert_eq!(sprite.get_char(0, 1).unwrap().chr, 'c');
        assert_eq!(sprite.get_char(1, 1).unwrap().chr, 'd');
        assert_eq!(sprite.get_char(2, 0), None);
        assert_eq!(sprite.get_char(0, 2), None);
    }

    #[test]
    fn fuzz_terminal_char_roundtrip() {
        let mut rng = rand::rng();

        for _ in 0..1000 {
            let u = rng.random_range(32u8..=126u8);
            let chr = char::from(u);

            let fg_color = if rng.random_bool(0.5) {
                Some(Color::rgb(
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                ))
            } else {
                None
            };

            let bg_color = if rng.random_bool(0.5) {
                Some(Color::rgb(
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                    rng.random_range(0..=255),
                ))
            } else {
                None
            };

            let pc = TerminalChar {
                chr,
                fg_color,
                bg_color,
            };

            let mut buf = Vec::new();
            pc.write_to(&mut buf).unwrap();
            let mut cur = std::io::Cursor::new(buf);
            let pc2 = TerminalChar::read_from(&mut cur).unwrap();
            assert_eq!(pc, pc2);
        }
    }

    #[test]
    fn fuzz_ascii_video_roundtrip() {
        let mut rng = rand::rng();

        for _ in 0..200 {
            let width = rng.random_range(1u16..5);
            let height = rng.random_range(1u16..5);
            let mut frames = Vec::new();

            for _ in 0..rng.random_range(1usize..5) {
                let mut frame = Vec::new();
                for _ in 0..(width * height) {
                    let u = rng.random_range(32u8..=126u8);
                    let chr = char::from(u);

                    let fg_color = if rng.random_bool(0.5) {
                        Some(Color::rgb(
                            rng.random_range(0..=255),
                            rng.random_range(0..=255),
                            rng.random_range(0..=255),
                        ))
                    } else {
                        None
                    };

                    let bg_color = if rng.random_bool(0.5) {
                        Some(Color::rgb(
                            rng.random_range(0..=255),
                            rng.random_range(0..=255),
                            rng.random_range(0..=255),
                        ))
                    } else {
                        None
                    };

                    frame.push(TerminalChar {
                        chr,
                        fg_color,
                        bg_color,
                    });
                }
                frames.push(AsciiSprite::new(width, height, frame).unwrap());
            }

            let video = AsciiVideo {
                width,
                height,
                frames,
            };
            let path = "test_fuzz_video.bin";
            video.write_to_file(path).unwrap();
            let loaded = AsciiVideo::read_from_file(path).unwrap();
            std::fs::remove_file(path).unwrap();
            assert_eq!(video, loaded);
        }
    }
}
