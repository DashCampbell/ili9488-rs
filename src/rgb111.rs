use embedded_graphics_core::pixelcolor::IntoStorage;
use embedded_graphics_core::prelude::{PixelColor, RgbColor};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Rgb111 {
    BLACK,
    RED,
    GREEN,
    BLUE,
    YELLOW,
    MAGENTA,
    CYAN,
    WHITE,
}
impl Rgb111 {
    /// Returns the color in binary form.
    /// Format `0bxxxxxrgb`
    pub fn raw(&self) -> u8 {
        match self {
            Self::BLACK => 0b000,
            Self::BLUE => 0b001,
            Self::CYAN => 0b011,
            Self::GREEN => 0b010,
            Self::MAGENTA => 0b101,
            Self::RED => 0b100,
            Self::WHITE => 0b111,
            Self::YELLOW => 0b110,
        }
    }
}
impl IntoStorage for Rgb111 {
    type Storage = u8;
    fn into_storage(self) -> Self::Storage {
        self.raw()
    }
}
impl PixelColor for Rgb111 {
    type Raw = ();
}
impl RgbColor for Rgb111 {
    const MAX_R: u8 = 1;
    const MAX_G: u8 = 1;
    const MAX_B: u8 = 1;
    const BLACK: Self = Self::BLACK;
    const RED: Self = Self::RED;
    const GREEN: Self = Self::GREEN;
    const BLUE: Self = Self::BLUE;
    const YELLOW: Self = Self::YELLOW;
    const MAGENTA: Self = Self::MAGENTA;
    const CYAN: Self = Self::CYAN;
    const WHITE: Self = Self::WHITE;
    fn r(&self) -> u8 {
        match self {
            Self::RED | Self::YELLOW | Self::MAGENTA | Self::WHITE => 1,
            _ => 0,
        }
    }
    fn g(&self) -> u8 {
        match self {
            Self::GREEN | Self::YELLOW | Self::CYAN | Self::WHITE => 1,
            _ => 0,
        }
    }
    fn b(&self) -> u8 {
        match self {
            Self::BLUE | Self::MAGENTA | Self::CYAN | Self::WHITE => 1,
            _ => 0,
        }
    }
}
