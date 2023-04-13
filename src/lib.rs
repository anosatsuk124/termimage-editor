use std::io::Write;
use std::ops::Deref;

use anyhow::{anyhow, Ok, Result};
use crossterm::{cursor, QueueableCommand};
use csv::Writer;
use thiserror::Error;

pub struct Brush(char);

impl Brush {
    pub const DEFAULT_BRUSH: Self = Self('█');
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Color(pub u8);

impl Color {
    /// The background color index.
    // NOTE: This is not configurable.
    pub const BG_COLOR: Self = Self(0);

    pub const MAX: u8 = u8::MAX;
}

impl Deref for Color {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u8> for Color {
    fn from(color: u8) -> Self {
        Self(color)
    }
}

impl From<Color> for u8 {
    fn from(src: Color) -> u8 {
        src.0
    }
}

// PERF: It is not the best way to store the buffer.
#[derive(Debug, Default)]
pub struct RawBuffer(Vec<Color>);

impl Deref for RawBuffer {
    type Target = Vec<Color>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub struct Buffer {
    /// The maximum position of the current rendered buffer.
    max_position: Position,
    data: RawBuffer,
}

impl Buffer {
    /// Parsing the buffer into csv format.
    /// FIXME: Should be serialization with serde.
    pub fn to_csv(&self, colors: &Colors) -> Result<String> {
        let mut csv = Writer::from_writer(Vec::new());

        for y in 0..self.max_position.y {
            for x in 0..self.max_position.x {
                let index = y * self.max_position.x + x;
                let color = self.data.0[index];
                let color_name = colors.0[u8::from(color) as usize].clone();
                csv.write_record(&[x.to_string(), y.to_string(), color_name])?;
            }
        }

        let string = String::from_utf8(csv.into_inner()?)?;

        Ok(string)
    }

    /// Return a new buffer with the new width.
    pub fn new_width_buffer(self, new_width: usize) -> Self {
        let mut new_data = RawBuffer::default();

        for y in 0..self.max_position.y {
            for x in 0..self.max_position.x {
                let index = y * self.max_position.x + x;
                new_data.0.push(self.data.0[index]);
            }

            for _ in 0..(new_width - self.max_position.x) {
                new_data.0.push(Color::BG_COLOR);
            }
        }

        Self {
            max_position: Position {
                x: new_width,
                y: self.max_position.y,
            },
            data: new_data,
        }
    }

    /// Return a new buffer with the new height.
    pub fn new_height_buffer(self, new_height: usize) -> Self {
        let mut new_data = RawBuffer::default();

        for y in 0..self.max_position.y {
            for x in 0..self.max_position.x {
                let index = y * self.max_position.x + x;
                new_data.0.push(self.data.0[index]);
            }
        }

        for _ in 0..(new_height - self.max_position.y) {
            for _ in 0..self.max_position.x {
                new_data.0.push(Color::BG_COLOR);
            }
        }

        Self {
            max_position: Position {
                x: self.max_position.x,
                y: new_height,
            },
            data: new_data,
        }
    }

    /// Return a new buffer with the new size.
    pub fn new_size_buffer(self, new_width: usize, new_height: usize) -> Self {
        self.new_width_buffer(new_width)
            .new_height_buffer(new_height)
    }

    /// NOTE: This does not check the range.
    pub fn get_index(&self, position: Position) -> usize {
        position.y * self.max_position.x + position.x
    }

    pub fn set_color(&mut self, position: Position, color: Color) -> Result<()> {
        let index = self.get_index(position);

        *self.data.0.get_mut(index).ok_or_else(|| {
            anyhow!(
                "The position is out of range: x: {}, y: {}",
                position.x,
                position.y
            )
        })? = color;

        Ok(())
    }

    pub fn get_color(&self, position: Position) -> Result<Color> {
        let index = self.get_index(position);

        self.data.0.get(index).copied().ok_or_else(|| {
            anyhow!(
                "The position is out of range: x: {}, y: {}",
                position.x,
                position.y
            )
        })
    }
}

/// The key is the color index.
pub struct Colors([String; Color::MAX as usize]);

#[derive(Debug, Clone)]
pub enum Mode {
    /// The cursor is in the normal mode.
    Normal,
    /// The cursor is in the selection mode.
    Selection,
    /// The cursor is in the draw mode.
    Draw {
        /// The color index.
        color: Color,
        brush: Option<char>,
    },
    /// The cursor is in the visual mode.
    Visual,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Error, Debug)]
pub enum ModeError {
    #[error("The cursor is not in the draw mode. Current mode is {0:?}")]
    NotDrawModeError(Mode),
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl From<(usize, usize)> for Position {
    fn from((x, y): (usize, usize)) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cursor {
    position: Position,
    mode: Mode,
}

impl Cursor {
    /// Return the current position of the cursor.
    pub fn current_position(&self) -> &Position {
        &self.position
    }

    /// Return the current mode of the cursor.
    pub fn current_mode(&self) -> &Mode {
        &self.mode
    }

    /// Return the new current with the new position.
    pub fn new_position_cursor(self, new_position: Position) -> Cursor {
        Self {
            position: new_position,
            mode: self.mode,
        }
    }

    /// Return the new current with the new mode.
    pub fn new_mode_cursor(self, new_mode: Mode) -> Cursor {
        Self {
            position: self.position,
            mode: new_mode,
        }
    }

    fn draw(&self, buffer: &mut Buffer) -> Result<()> {
        if let Mode::Draw { color, .. } = self.mode {
            buffer.set_color(self.position, color)?;
            return Ok(());
        }

        Err(ModeError::NotDrawModeError(self.mode.clone()).into())
    }
}

pub trait Renderable: Write {
    fn render(&mut self, buffer: &Buffer) -> Result<()> {
        let pos = self.size()?;
        for y in 0..pos.y {
            for x in 0..pos.x {
                let pos = Position { x, y };
                let color = buffer.get_color(pos)?;
                unimplemented!("TODO: Write the color to the output.")
            }
        }
        Ok(())
    }
    fn scroll(&mut self) -> Result<()> {
        Ok(())
    }
    fn size(&mut self) -> Result<Position>;
    fn set_position(&mut self, position: Position) -> Result<()>;
}

// impl<T: Write + ?Sized> Renderable for T {
//     fn size(&mut self) -> Result<Position> {
//         todo!()
//     }
// }
