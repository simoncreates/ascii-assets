pub use crossterm::style::Color;

use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, PartialEq, Clone)]
pub struct TerminalChar {
    pub chr: char,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl TerminalChar {
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.chr as u32)?;
        if let Some(Color::Rgb { r, g, b }) = self.fg_color{
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
        let fg_color= {
            let has_fg_color= r.read_u8()?;
            if has_fg_color== 1 {
                let red = r.read_u8()?;
                let green = r.read_u8()?;
                let blue= r.read_u8()?;
                Some(Color::Rgb { r: red, g: green, b: blue })
            } else {
                None
            }
        };
        let bg_color = {
            let has_bg_color = r.read_u8()?;
            if has_bg_color == 1{
                let red = r.read_u8()?;
                let green = r.read_u8()?;
                let blue = r.read_u8()?;
                Some(Color::Rgb { r: red, g: green, b: blue })
            } else {
                None
            }
        };
        Ok(TerminalChar { chr, fg_color, bg_color })
    }
}

#[derive(Debug, PartialEq)]
pub struct AsciiVideo {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<Vec<TerminalChar>>,
}

impl AsciiVideo {
    const MAGIC: [u8; 4] = *b"ASCV";
    const VERSION: u8 = 1;

    /// write a complete video to a file
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
            for pix in frame {
                pix.write_to(&mut w)?;
            }
        }
        w.flush()?;
        Ok(())
    }
    pub fn read_from_file(path: &str) -> io::Result<Self> {
        let f = File::open(path)?;
        let mut r = BufReader::new(f);
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        assert_eq!(&magic, &Self::MAGIC, "Bad magic");
        let ver = r.read_u8()?;
        assert_eq!(ver, Self::VERSION, "Unsupported version");
        let width = r.read_u16::<LittleEndian>()?;
        let height = r.read_u16::<LittleEndian>()?;
        let frame_count = r.read_u32::<LittleEndian>()? as usize;
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let mut frame = Vec::with_capacity((width as usize) * (height as usize));
            for _ in 0..(width as usize * height as usize) {
                frame.push(TerminalChar::read_from(&mut r)?);
            }
            frames.push(frame);
        }
        Ok(AsciiVideo { width, height, frames })
    }
    pub fn get_frame(&self, index: usize) -> Option<Vec<Vec<TerminalChar>>> {
        self.frames.get(index).map(|flat| {
            let mut grid = Vec::with_capacity(self.height as usize);
            for row in 0..self.height {
                let mut row_vec = Vec::with_capacity(self.width as usize);
                for col in 0..self.width {
                    let idx = (row as usize) * (self.width as usize) + (col as usize);
                    row_vec.push(flat[idx].clone());
                }
                grid.push(row_vec);
            }
            grid
        })
    }

    /// converts all frames to a 3D Vec: Vec<Frame<Rows<Cols<TerminalChar>>>>
    pub fn frames_as_grid(&self) -> Vec<Vec<Vec<TerminalChar>>> {
        (0..self.frames.len())
            .filter_map(|i| self.get_frame(i))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn packed_char_roundtrip() {
        let pc = TerminalChar { chr: '@', fg_color: Some(Color::Rgb { r: 1, g: 2, b: 3 }), bg_color: None };
        let mut buf = Vec::new();
        pc.write_to(&mut buf).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        let pc2 = TerminalChar::read_from(&mut cur).unwrap();
        assert_eq!(pc, pc2);
    }

    #[test]
    fn video_roundtrip() {
        let width = 2;
        let height = 2;
        let frame = vec![
            TerminalChar { chr: 'A', fg_color: None, bg_color: None },
            TerminalChar { chr: 'B', fg_color: None, bg_color: None },
            TerminalChar { chr: 'C', fg_color: None, bg_color: None },
            TerminalChar { chr: 'D', fg_color: None, bg_color: None },
        ];
        let video = AsciiVideo { width, height, frames: vec![frame.clone(), frame] };
        let path = "test_video.bin";
        video.write_to_file(path).unwrap();
        let loaded = AsciiVideo::read_from_file(path).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(video, loaded);
    }
}
