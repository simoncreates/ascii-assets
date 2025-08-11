use crossterm::style::Color;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TerminalChar {
    pub chr: char,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl TerminalChar {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.chr as u32)?;
        if let Some(Color::Rgb { r, g, b }) = self.fg_color {
            w.write_u8(1)?;
            w.write_u8(r)?;
            w.write_u8(g)?;
            w.write_u8(b)?;
        } else {
            w.write_u8(0)?;
        }
        if let Some(Color::Rgb { r, g, b }) = self.bg_color {
            w.write_u8(1)?;
            w.write_u8(r)?;
            w.write_u8(g)?;
            w.write_u8(b)?;
        } else {
            w.write_u8(0)?;
        }
        Ok(())
    }

    pub fn read_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let code = r.read_u32::<LittleEndian>()?;
        let chr = std::char::from_u32(code).unwrap();
        let fg_color = {
            let has_fg_color = r.read_u8()?;
            if has_fg_color == 1 {
                let red = r.read_u8()?;
                let green = r.read_u8()?;
                let blue = r.read_u8()?;
                Some(Color::Rgb {
                    r: red,
                    g: green,
                    b: blue,
                })
            } else {
                None
            }
        };
        let bg_color = {
            let has_bg_color = r.read_u8()?;
            if has_bg_color == 1 {
                let red = r.read_u8()?;
                let green = r.read_u8()?;
                let blue = r.read_u8()?;
                Some(Color::Rgb {
                    r: red,
                    g: green,
                    b: blue,
                })
            } else {
                None
            }
        };
        Ok(TerminalChar {
            chr,
            fg_color,
            bg_color,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct AsciiVideo {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<AsciiSprite>,
}

impl AsciiVideo {
    const MAGIC: [u8; 4] = *b"ASCV";
    const VERSION: u8 = 1;

    /// Creates a new video from frames, checking all are same size.
    pub fn new(width: u16, height: u16, frames: Vec<AsciiSprite>) -> io::Result<Self> {
        for (i, frame) in frames.iter().enumerate() {
            if frame.width != width || frame.height != height {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Frame {} has size {}x{} but expected {}x{}",
                        i, frame.width, frame.height, width, height
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

    pub fn write_to_file(&self, path: &str) -> io::Result<()> {
        let f = File::create(path)?;
        let mut w = BufWriter::new(f);

        // header
        w.write_all(&Self::MAGIC)?;
        w.write_u8(Self::VERSION)?;
        w.write_u16::<LittleEndian>(self.width)?;
        w.write_u16::<LittleEndian>(self.height)?;
        w.write_u32::<LittleEndian>(self.frames.len() as u32)?;

        // frames
        for frame in &self.frames {
            frame.write_to(&mut w)?;
        }

        w.flush()?;
        Ok(())
    }

    pub fn read_from_file(path: &str) -> io::Result<Self> {
        let f = File::open(path)?;
        let mut r = BufReader::new(f);

        // header
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        assert_eq!(&magic, &Self::MAGIC, "Bad magic");

        let ver = r.read_u8()?;
        assert_eq!(ver, Self::VERSION, "Unsupported version");

        let width = r.read_u16::<LittleEndian>()?;
        let height = r.read_u16::<LittleEndian>()?;
        let frame_count = r.read_u32::<LittleEndian>()? as usize;

        // frames
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            frames.push(AsciiSprite::read_from(&mut r, width, height)?);
        }

        Self::new(width, height, frames)
    }

    pub fn get_frame(&self, index: usize) -> Option<Vec<Vec<TerminalChar>>> {
        self.frames.get(index).map(|sprite| {
            let mut grid = Vec::with_capacity(sprite.height as usize);
            for row in 0..sprite.height {
                let mut row_vec = Vec::with_capacity(sprite.width as usize);
                for col in 0..sprite.width {
                    let idx = (row as usize) * (sprite.width as usize) + (col as usize);
                    row_vec.push(sprite.pixels[idx]);
                }
                grid.push(row_vec);
            }
            grid
        })
    }

    pub fn frames_as_grid(&self) -> Vec<Vec<Vec<TerminalChar>>> {
        (0..self.frames.len())
            .filter_map(|i| self.get_frame(i))
            .collect()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AsciiSprite {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<TerminalChar>,
}

impl AsciiSprite {
    pub fn new(width: u16, height: u16, pixels: Vec<TerminalChar>) -> io::Result<Self> {
        if pixels.len() != (width as usize) * (height as usize) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Sprite pixel count ({}) does not match width*height ({}).",
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

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        for pix in &self.pixels {
            pix.write_to(w)?;
        }
        Ok(())
    }

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

    pub fn as_grid(&self) -> Vec<Vec<TerminalChar>> {
        let mut grid = Vec::with_capacity(self.height as usize);
        for row in 0..self.height {
            let mut row_vec = Vec::with_capacity(self.width as usize);
            for col in 0..self.width {
                let idx = (row as usize) * (self.width as usize) + (col as usize);
                row_vec.push(self.pixels[idx]);
            }
            grid.push(row_vec);
        }
        grid
    }

    pub fn get_char(&self, x: u16, y: u16) -> Option<TerminalChar> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        Some(self.pixels[idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use rand::Rng;

        let mut rng = rand::rng();

        for _ in 0..100 {
            let chr = rng.random_range(32u8..127u8) as char;
            let fg_color = if rng.random_bool(0.5) {
                Some(Color::Rgb {
                    r: rng.random(),
                    g: rng.random(),
                    b: rng.random(),
                })
            } else {
                None
            };
            let bg_color = if rng.random_bool(0.5) {
                Some(Color::Rgb {
                    r: rng.random(),
                    g: rng.random(),
                    b: rng.random(),
                })
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
        use rand::Rng;

        let mut rng = rand::rng();

        for _ in 0..20 {
            let width = rng.random_range(1..5);
            let height = rng.random_range(1..5);
            let mut frames = Vec::new();

            for _ in 0..rng.random_range(1..5) {
                let mut frame = Vec::new();
                for _ in 0..(width * height) {
                    let chr = rng.random_range(32u8..127u8) as char;
                    let fg_color = if rng.random_bool(0.5) {
                        Some(Color::Rgb {
                            r: rng.random(),
                            g: rng.random(),
                            b: rng.random(),
                        })
                    } else {
                        None
                    };
                    let bg_color = if rng.random_bool(0.5) {
                        Some(Color::Rgb {
                            r: rng.random(),
                            g: rng.random(),
                            b: rng.random(),
                        })
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
