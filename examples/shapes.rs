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
use embedded_hal_bus::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    pixelcolor::{Rgb666, RgbColor},
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
};

use ili9488_rs::{Ili9488, Orientation, Rgb111, Rgb666Mode};

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
    let miso = p.PB4;

    let spi = Spi::new_blocking(peri, sclk, mosi, miso, spi_config);
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

    // Setup shapes
    let rect_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb666::RED)
        .stroke_width(3)
        .fill_color(Rgb666::GREEN)
        .build();
    let tri_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb666::BLUE)
        .stroke_width(3)
        .fill_color(Rgb666::RED)
        .build();
    let circle_style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb666::CYAN)
        .stroke_width(3)
        .build();
    let rect = Rectangle::new(Point::new(30, 20), Size::new(120, 80)).into_styled(rect_style);
    let triangle = Triangle::new(
        Point::new(300, 20),
        Point::new(250, 100),
        Point::new(350, 100),
    )
    .into_styled(tri_style);
    let circle = Circle::new(Point::new(200, 200), 100).into_styled(circle_style);

    // Render
    display.clear_screen_fast(Rgb111::BLACK).unwrap();

    rect.draw(&mut display).unwrap();
    triangle.draw(&mut display).unwrap();
    circle.draw(&mut display).unwrap();

    loop {}
}
