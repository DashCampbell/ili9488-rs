#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::Delay;
use embedded_graphics::mono_font::iso_8859_14::FONT_10X20;
use embedded_graphics::pixelcolor::Rgb666;
use embedded_hal_bus::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};

use ili9488_rs::{Ili9488, Orientation, Rgb666Mode};

// #[embassy_executor::main]
#[entry]
fn main() -> ! {
    let mut config = Config::default();
    {
        // Configure the system clock to be 80 MHz
        use embassy_stm32::rcc::*;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.hsi = true;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI, // 16MHz
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL10,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
    }
    let p = embassy_stm32::init(config);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = Hertz::mhz(40);
    spi_config.miso_pull = Pull::Up;
    spi_config.rise_fall_speed = Speed::VeryHigh;

    let peri = p.SPI3;
    let sclk = p.PB3;
    let mosi = p.PB5;

    let spi = Spi::new_txonly(peri, sclk, mosi, p.DMA2_CH2, spi_config);
    let cs = Output::new(p.PA0, Level::High, embassy_stm32::gpio::Speed::VeryHigh);
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let dc = Output::new(p.PA1, Level::Low, embassy_stm32::gpio::Speed::VeryHigh);

    let spi_interface = SPIInterface::new(spi_device, dc);
    let reset_pin = Output::new(p.PA11, Level::Low, embassy_stm32::gpio::Speed::VeryHigh);
    let mut delay = Delay;

    info!("Initializing Display...");
    let mut display = Ili9488::new(
        spi_interface,
        reset_pin,
        &mut delay,
        Orientation::LandscapeFlipped,
        Rgb666Mode,
    )
    .unwrap();
    info!("Done");

    // Create styles used by the drawing operations.
    let thin_stroke = PrimitiveStyle::with_stroke(Rgb666::RED, 2);
    let thick_stroke = PrimitiveStyle::with_stroke(Rgb666::BLUE, 10);
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb666::BLUE)
        .stroke_width(20)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let fill = PrimitiveStyle::with_fill(Rgb666::CSS_GREEN);
    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb666::CSS_PURPLE);

    // Draw a 3px wide outline around the display.
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)
        .unwrap();

    // Draw a triangle.
    let yoffset = 60;
    let i = 80;
    Triangle::new(
        Point::new(i, i + yoffset),
        Point::new(i + i, i + yoffset),
        Point::new(i + i / 2, yoffset),
    )
    .into_styled(thin_stroke)
    .draw(&mut display)
    .unwrap();

    // Draw a filled square
    Rectangle::new(Point::new(200, yoffset), Size::new(i as u32, i as u32))
        .into_styled(fill)
        .draw(&mut display)
        .unwrap();

    // Draw a circle with a 3px wide stroke.
    Circle::new(Point::new(320, yoffset), (i + 1) as u32)
        .into_styled(thick_stroke)
        .draw(&mut display)
        .unwrap();

    // Draw centered text.
    let text = "embedded-graphics";
    Text::with_alignment(
        text,
        display.bounding_box().center() + Point::new(0, 15),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    loop {}
}
