#![no_std]

//! ILI9488 Display Driver
//!
//! ### Usage
//!
//! To control the display you need to set up:
//!
//! * Interface for communicating with display ([display-interface-spi crate] for SPI)
//! * Configuration (reset pin, delay, orientation and size) for display
//!
//! ```ignore
//! let iface = SPIInterface::new(spi, dc, cs);
//!
//! let mut display = Ili9341::new(
//!     iface,
//!     reset_gpio,
//!     &mut delay,
//!     Orientation::Landscape,
//!     ili9341::DisplaySize240x320,
//! )
//! .unwrap();
//!
//! display.clear(Rgb565::RED).unwrap()
//! ```
//!
//! [display-interface-spi crate]: https://crates.io/crates/display-interface-spi
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

use display_interface::{DataFormat, WriteOnlyDataCommand};

use embedded_graphics_core::pixelcolor::{IntoStorage, Rgb565, Rgb666};
use embedded_graphics_core::prelude::RgbColor;

mod graphics_core;
mod rgb111;
pub use crate::rgb111::*;
pub use display_interface::DisplayError;

type Result<T = (), E = DisplayError> = core::result::Result<T, E>;

/// Trait that defines display size information
pub trait DisplaySize {
    /// Width in pixels
    const WIDTH: usize;
    /// Height in pixels
    const HEIGHT: usize;
}

/// Generic display size of 320x480 pixels
pub struct DisplaySize320x480;

impl DisplaySize for DisplaySize320x480 {
    const WIDTH: usize = 320;
    const HEIGHT: usize = 480;
}

/// Trait for Valid Pixel Formats for the ILI9488
/// Implemented by [Rgb111Mode] & [Rgb666Mode]
pub trait Ili9488PixelFormat: Copy + Clone {
    /// The data used for the PixelFormatSet command
    const DATA: u8;
}

/// 3 bpp
#[derive(Copy, Clone)]
pub struct Rgb111Mode;

impl Ili9488PixelFormat for Rgb111Mode {
    const DATA: u8 = 0x1;
}
/// 18 bpp
#[derive(Copy, Clone)]
pub struct Rgb666Mode;
impl Ili9488PixelFormat for Rgb666Mode {
    const DATA: u8 = 0x66;
}

/// Trait implementation for writing different pixel formats to the ili9488's memory
pub trait Ili9488MemoryWrite {
    type PixelFormat: RgbColor;
    fn write_iter<I: IntoIterator<Item = Self::PixelFormat>>(&mut self, data: I) -> Result;
    fn write_slice(&mut self, data: &[Self::PixelFormat]) -> Result;
}

/// For quite a few boards (ESP32-S2-Kaluga-1, M5Stack, M5Core2 and others),
/// the ILI9341 initialization command arguments are slightly different
///
/// This trait provides the flexibility for users to define their own
/// initialization command arguments suitable for the particular board they are using
pub trait Mode {
    fn mode(&self) -> u8;

    fn is_landscape(&self) -> bool;
}

/// The default implementation of the Mode trait from above
/// Should work for most (but not all) boards
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}

impl Mode for Orientation {
    fn mode(&self) -> u8 {
        match self {
            Self::Portrait => 0x40 | 0x08,
            Self::Landscape => 0x20 | 0x08,
            Self::PortraitFlipped => 0x80 | 0x08,
            Self::LandscapeFlipped => 0x40 | 0x80 | 0x20 | 0x08,
        }
    }

    fn is_landscape(&self) -> bool {
        match self {
            Self::Landscape | Self::LandscapeFlipped => true,
            Self::Portrait | Self::PortraitFlipped => false,
        }
    }
}

/// Specify state of specific mode of operation
pub enum ModeState {
    On,
    Off,
}

/// In 4-wire spi mode, only RGB111 or RGB666 data formats are supported
///
/// There are two method for drawing to the screen:
/// [Ili9341::draw_raw_iter] and [Ili9341::draw_raw_slice]
///
/// In both cases the expected pixel format is rgb565.
///
/// The hardware makes it efficient to draw rectangles on the screen.
///
/// What happens is the following:
///
/// - A drawing window is prepared (with the 2 opposite corner coordinates)
/// - The starting point for drawint is the top left corner of this window
/// - Every pair of bytes received is intepreted as a pixel value in rgb565
/// - As soon as a pixel is received, an internal counter is incremented,
///   and the next word will fill the next pixel (the adjacent on the right, or
///   the first of the next row if the row ended)
pub struct Ili9488<IFACE, RESET, PixelFormat> {
    interface: IFACE,
    reset: RESET,
    width: usize,
    height: usize,
    landscape: bool,
    _pixel_format: PixelFormat,
}

impl<IFACE, RESET, PixelFormat> Ili9488<IFACE, RESET, PixelFormat>
where
    IFACE: WriteOnlyDataCommand,
    RESET: OutputPin,
    PixelFormat: Ili9488PixelFormat,
{
    pub fn new<DELAY, MODE>(
        interface: IFACE,
        reset: RESET,
        delay: &mut DELAY,
        orientation: MODE,
        pixel_format: PixelFormat,
    ) -> Result<Self>
    where
        DELAY: DelayNs,
        MODE: Mode,
    {
        let mut ili9488 = Self {
            interface,
            reset,
            width: DisplaySize320x480::WIDTH,
            height: DisplaySize320x480::HEIGHT,
            landscape: false,
            _pixel_format: pixel_format,
        };

        // Put SPI bus in known state for TFT with CS tied low
        ili9488.command(Command::NOP, &[])?;

        ili9488
            .reset
            .set_high()
            .map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(5);

        // Do hardware reset by holding reset low for at least 10us
        ili9488.reset.set_low().map_err(|_| DisplayError::RSError)?;
        let _ = delay.delay_ms(20);

        // Set high for normal operation
        ili9488
            .reset
            .set_high()
            .map_err(|_| DisplayError::RSError)?;

        // Wait for reset to complete
        let _ = delay.delay_ms(150);

        // Do software reset
        ili9488.command(Command::SoftwareReset, &[])?;

        // Wait 5ms after reset before sending commands
        // and 120ms before sending Sleep Out
        let _ = delay.delay_ms(150);

        // Initialization Sequence, taken from (https://github.com/Bodmer/TFT_eSPI/blob/master/TFT_Drivers/ILI9488_Init.h)

        // Positive Gamma Control
        ili9488.command(
            Command::PositiveGammaControl,
            &[
                0x00, 0x03, 0x09, 0x08, 0x16, 0x0A, 0x3F, 0x78, 0x4C, 0x09, 0x0A, 0x08, 0x16, 0x1A,
                0x0F,
            ],
        )?;

        // Negative Gamma Control
        ili9488.command(
            Command::NegativeGammaControl,
            &[
                0x00, 0x16, 0x19, 0x03, 0x0F, 0x05, 0x32, 0x45, 0x46, 0x04, 0x0E, 0x0D, 0x35, 0x37,
                0x0F,
            ],
        )?;

        ili9488.command(Command::PowerControl1, &[0x17, 0x15])?;

        ili9488.command(Command::PowerControl2, &[0x41])?;

        ili9488.command(Command::VCOMControl, &[0x00, 0x12, 0x80])?;

        ili9488.command(Command::MemoryAccessControl, &[0x48])?; // MX, BGR

        ili9488.command(Command::PixelFormatSet, &[PixelFormat::DATA])?;

        ili9488.command(Command::InterfaceModeControl, &[0x00])?;

        ili9488.command(Command::NormalModeFrameRate, &[0xA0])?;

        ili9488.command(Command::DisplayInversionControl, &[0x02])?;

        ili9488.command(Command::DisplayFunctionControl, &[0x02, 0x02, 0x3B])?;

        ili9488.command(Command::EntryModeSet, &[0xC6])?;

        ili9488.command(Command::AdjustControl3, &[0xA9, 0x51, 0x2C, 0x82])?;

        ili9488.sleep_mode(ModeState::Off)?;

        ili9488.set_orientation(orientation)?;

        ili9488.display_mode(ModeState::On)?;

        Ok(ili9488)
    }
}

impl<IFACE, RESET, PixelFormat> Ili9488<IFACE, RESET, PixelFormat>
where
    IFACE: WriteOnlyDataCommand,
    PixelFormat: Ili9488PixelFormat,
{
    pub fn change_pixel_format<P: Ili9488PixelFormat>(
        mut self,
        pixel_format: P,
    ) -> Result<Ili9488<IFACE, RESET, P>> {
        self.command(Command::PixelFormatSet, &[P::DATA])?;

        Ok(Ili9488 {
            interface: self.interface,
            reset: self.reset,
            width: self.width,
            height: self.height,
            landscape: self.landscape,
            _pixel_format: pixel_format,
        })
    }
    fn command(&mut self, cmd: Command, args: &[u8]) -> Result {
        self.interface.send_commands(DataFormat::U8(&[cmd as u8]))?;
        self.interface.send_data(DataFormat::U8(args))
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result {
        self.command(
            Command::ColumnAddressSet,
            &[
                (x0 >> 8) as u8,
                (x0 & 0xff) as u8,
                (x1 >> 8) as u8,
                (x1 & 0xff) as u8,
            ],
        )?;
        self.command(
            Command::PageAddressSet,
            &[
                (y0 >> 8) as u8,
                (y0 & 0xff) as u8,
                (y1 >> 8) as u8,
                (y1 & 0xff) as u8,
            ],
        )
    }

    /// Configures the screen for hardware-accelerated vertical scrolling.
    pub fn configure_vertical_scroll(
        &mut self,
        fixed_top_lines: u16,
        fixed_bottom_lines: u16,
    ) -> Result<Scroller> {
        let height = if self.landscape {
            self.width
        } else {
            self.height
        } as u16;
        let scroll_lines = height as u16 - fixed_top_lines - fixed_bottom_lines;

        self.command(
            Command::VerticalScrollDefine,
            &[
                (fixed_top_lines >> 8) as u8,
                (fixed_top_lines & 0xff) as u8,
                (scroll_lines >> 8) as u8,
                (scroll_lines & 0xff) as u8,
                (fixed_bottom_lines >> 8) as u8,
                (fixed_bottom_lines & 0xff) as u8,
            ],
        )?;

        Ok(Scroller::new(fixed_top_lines, fixed_bottom_lines, height))
    }

    pub fn scroll_vertically(&mut self, scroller: &mut Scroller, num_lines: u16) -> Result {
        scroller.top_offset += num_lines;
        if scroller.top_offset > (scroller.height - scroller.fixed_bottom_lines) {
            scroller.top_offset = scroller.fixed_top_lines
                + (scroller.top_offset + scroller.fixed_bottom_lines - scroller.height)
        }

        self.command(
            Command::VerticalScrollAddr,
            &[
                (scroller.top_offset >> 8) as u8,
                (scroller.top_offset & 0xff) as u8,
            ],
        )
    }

    /// Change the orientation of the screen
    pub fn set_orientation<MODE>(&mut self, orientation: MODE) -> Result
    where
        MODE: Mode,
    {
        self.command(Command::MemoryAccessControl, &[orientation.mode()])?;

        if self.landscape ^ orientation.is_landscape() {
            core::mem::swap(&mut self.height, &mut self.width);
        }
        self.landscape = orientation.is_landscape();
        Ok(())
    }

    /// Control the screen sleep mode:
    pub fn sleep_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::SleepModeOn, &[]),
            ModeState::Off => self.command(Command::SleepModeOff, &[]),
        }
    }

    /// Control the screen display mode
    pub fn display_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::DisplayOn, &[]),
            ModeState::Off => self.command(Command::DisplayOff, &[]),
        }
    }

    /// Invert the pixel color on screen
    pub fn invert_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::InvertOn, &[]),
            ModeState::Off => self.command(Command::InvertOff, &[]),
        }
    }

    /// Idle mode reduces the number of colors to 8
    pub fn idle_mode(&mut self, mode: ModeState) -> Result {
        match mode {
            ModeState::On => self.command(Command::IdleModeOn, &[]),
            ModeState::Off => self.command(Command::IdleModeOff, &[]),
        }
    }

    /// Set display brightness to the value between 0 and 255
    pub fn brightness(&mut self, brightness: u8) -> Result {
        self.command(Command::SetBrightness, &[brightness])
    }

    /// Set adaptive brightness value equal to [AdaptiveBrightness]
    pub fn content_adaptive_brightness(&mut self, value: AdaptiveBrightness) -> Result {
        self.command(Command::ContentAdaptiveBrightness, &[value as _])
    }

    /// Configure [FrameRateClockDivision] and [FrameRate] in normal mode
    pub fn normal_mode_frame_rate(
        &mut self,
        clk_div: FrameRateClockDivision,
        frame_rate: FrameRate,
    ) -> Result {
        self.command(
            Command::NormalModeFrameRate,
            &[clk_div as _, frame_rate as _],
        )
    }

    /// Configure [FrameRateClockDivision] and [FrameRate] in idle mode
    pub fn idle_mode_frame_rate(
        &mut self,
        clk_div: FrameRateClockDivision,
        frame_rate: FrameRate,
    ) -> Result {
        self.command(Command::IdleModeFrameRate, &[clk_div as _, frame_rate as _])
    }
}

impl<IFACE, RESET> Ili9488MemoryWrite for Ili9488<IFACE, RESET, Rgb666Mode>
where
    IFACE: WriteOnlyDataCommand,
{
    type PixelFormat = Rgb666;

    fn write_iter<I: IntoIterator<Item = Self::PixelFormat>>(&mut self, data: I) -> Result {
        self.command(Command::MemoryWrite, &[])?;
        for color in data {
            self.interface.send_data(DataFormat::U8(&[
                color.r() << 2,
                color.g() << 2,
                color.b() << 2,
            ]))?;
        }
        Ok(())
    }
    fn write_slice(&mut self, data: &[Self::PixelFormat]) -> Result {
        self.command(Command::MemoryWrite, &[])?;
        for color in data {
            self.interface.send_data(DataFormat::U8(&[
                color.r() << 2,
                color.g() << 2,
                color.b() << 2,
            ]))?;
        }
        Ok(())
    }
}
impl<IFACE, RESET> Ili9488MemoryWrite for Ili9488<IFACE, RESET, Rgb111Mode>
where
    IFACE: WriteOnlyDataCommand,
{
    type PixelFormat = Rgb111;
    // TODO: Fix implementations
    fn write_iter<I: IntoIterator<Item = Self::PixelFormat>>(&mut self, data: I) -> Result {
        self.command(Command::MemoryWrite, &[])?;

        let mut data = data.into_iter();
        while let Some(p1) = data.next() {
            self.interface
                .send_data(DataFormat::U8(&[(p1.into_storage() << 3)
                    | (data.next().map(|p| p.into_storage()).unwrap_or_default())]))?;
        }
        Ok(())
    }
    fn write_slice(&mut self, data: &[Self::PixelFormat]) -> Result {
        self.command(Command::MemoryWrite, &[])?;
        self.interface
            .send_data(DataFormat::U8Iter(&mut data.chunks(2).map(|pixels| {
                (pixels[0].raw() << 3) | pixels.get(1).map(|p| p.into_storage()).unwrap_or_default()
            })))?;
        Ok(())
    }
}

impl<IFACE, RESET> Ili9488<IFACE, RESET, Rgb666Mode>
where
    IFACE: WriteOnlyDataCommand,
{
    /// Draw a raw RGB565 image buffer to the display in RGB666 mode.
    /// `data` should be a slice of u16 values in RGB565 format.
    /// The rectangle is defined by (x0, y0) to (x1, y1) inclusive.
    pub fn draw_rgb565_image(
        &mut self,
        x0: u16,
        y0: u16,
        width: u16,
        height: u16,
        data: &[u16],
    ) -> Result {
        self.set_window(x0, y0, x0 + width, y0 + height)?;
        self.write_iter(data.iter().map(|col| {
            Rgb666::from(Rgb565::new(
                ((col & (0b11111 << 11)) >> 11) as u8,
                ((col & (0b111111 << 5)) >> 5) as u8,
                (col & 0b11111) as u8,
            ))
        }))
    }
}
impl<IFACE, RESET, PixelFormat> Ili9488<IFACE, RESET, PixelFormat>
where
    Self: Ili9488MemoryWrite,
    IFACE: WriteOnlyDataCommand,
    PixelFormat: Ili9488PixelFormat,
{
    pub fn draw_raw_iter<
        I: IntoIterator<
            Item = <Ili9488<IFACE, RESET, PixelFormat> as Ili9488MemoryWrite>::PixelFormat,
        >,
    >(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: I,
    ) -> Result {
        self.set_window(x0, y0, x1, y1)?;
        self.write_iter(data)
    }
    /// Draw a rectangle on the screen, represented by top-left corner (x0, y0)
    /// and bottom-right corner (x1, y1).
    ///
    /// The border is included.
    ///
    /// This method accepts a raw buffer of words that will be copied to the screen
    /// video memory.
    pub fn draw_raw_slice(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        data: &[<Ili9488<IFACE, RESET, PixelFormat> as Ili9488MemoryWrite>::PixelFormat],
    ) -> Result {
        self.set_window(x0, y0, x1, y1)?;
        self.write_slice(data)
    }
    /// Fill entire screen with specfied color
    pub fn clear_screen(
        &mut self,
        color: <Ili9488<IFACE, RESET, PixelFormat> as Ili9488MemoryWrite>::PixelFormat,
    ) -> Result {
        let color = core::iter::repeat(color).take(self.width * self.height);
        self.draw_raw_iter(0, 0, self.width as u16, self.height as u16, color)
    }
    /// Fast way to fill entire screen, only uses 3 bits per pixel (bpp)
    pub fn clear_screen_fast(&mut self, color: Rgb111) -> Result {
        // Switch pixel format to 3 bpp
        if PixelFormat::DATA != Rgb111Mode::DATA {
            self.command(Command::PixelFormatSet, &[Rgb111Mode::DATA])?;
        }

        // Clear the screen with 3 bpp
        let color = (color.into_storage() << 3) | color.into_storage();
        let mut data = core::iter::repeat(color).take(self.width * self.height / 2);

        self.set_window(0, 0, self.width as u16, self.height as u16)?;
        self.command(Command::MemoryWrite, &[])?;
        self.interface.send_data(DataFormat::U8Iter(&mut data))?;

        // Switch back to original pixel format
        if PixelFormat::DATA != Rgb111Mode::DATA {
            self.command(Command::PixelFormatSet, &[PixelFormat::DATA])
        } else {
            Ok(())
        }
    }
}

impl<IFACE, RESET, PixelFormat> Ili9488<IFACE, RESET, PixelFormat> {
    /// Get the current screen width. It can change based on the current orientation
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the current screen heighth. It can change based on the current orientation
    pub fn height(&self) -> usize {
        self.height
    }
    /// Consumes the ILI9488, gives back the interface and reset peripherals
    pub fn release(self) -> (IFACE, RESET) {
        (self.interface, self.reset)
    }
}

/// Scroller must be provided in order to scroll the screen. It can only be obtained
/// by configuring the screen for scrolling.
pub struct Scroller {
    top_offset: u16,
    fixed_bottom_lines: u16,
    fixed_top_lines: u16,
    height: u16,
}

impl Scroller {
    fn new(fixed_top_lines: u16, fixed_bottom_lines: u16, height: u16) -> Scroller {
        Scroller {
            top_offset: fixed_top_lines,
            fixed_top_lines,
            fixed_bottom_lines,
            height,
        }
    }
}

/// Available Adaptive Brightness values
pub enum AdaptiveBrightness {
    Off = 0x00,
    UserInterfaceImage = 0x01,
    StillPicture = 0x02,
    MovingImage = 0x03,
}

/// Available frame rate in Hz
pub enum FrameRate {
    FrameRate119 = 0x10,
    FrameRate112 = 0x11,
    FrameRate106 = 0x12,
    FrameRate100 = 0x13,
    FrameRate95 = 0x14,
    FrameRate90 = 0x15,
    FrameRate86 = 0x16,
    FrameRate83 = 0x17,
    FrameRate79 = 0x18,
    FrameRate76 = 0x19,
    FrameRate73 = 0x1a,
    FrameRate70 = 0x1b,
    FrameRate68 = 0x1c,
    FrameRate65 = 0x1d,
    FrameRate63 = 0x1e,
    FrameRate61 = 0x1f,
}

/// Frame rate clock division
pub enum FrameRateClockDivision {
    Fosc = 0x00,
    FoscDiv2 = 0x01,
    FoscDiv4 = 0x02,
    FoscDiv8 = 0x03,
}

#[derive(Clone, Copy)]
enum Command {
    NOP = 0x00,
    SoftwareReset = 0x01,
    SleepModeOn = 0x10,
    SleepModeOff = 0x11,
    InvertOff = 0x20,
    InvertOn = 0x21,
    DisplayOff = 0x28,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2a,
    PageAddressSet = 0x2b,
    MemoryWrite = 0x2c,
    VerticalScrollDefine = 0x33,
    MemoryAccessControl = 0x36,
    VerticalScrollAddr = 0x37,
    IdleModeOff = 0x38,
    IdleModeOn = 0x39,
    PixelFormatSet = 0x3a,
    SetBrightness = 0x51,
    ContentAdaptiveBrightness = 0x55,
    InterfaceModeControl = 0xb0,
    NormalModeFrameRate = 0xb1,
    IdleModeFrameRate = 0xb2,
    DisplayInversionControl = 0xb4,
    DisplayFunctionControl = 0xb6,
    EntryModeSet = 0xb7,
    PowerControl1 = 0xc0,
    PowerControl2 = 0xc1,
    VCOMControl = 0xc5,
    PositiveGammaControl = 0xe0,
    NegativeGammaControl = 0xe1,
    AdjustControl3 = 0xf7,
}
