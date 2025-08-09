#![no_std]
#![no_main]

use defmt::*;
use display_interface_spi::SPIInterface;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::{Delay, Instant, Timer};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_framebuf::FrameBuf;
use embedded_hal_bus::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    pixelcolor::{Rgb666, RgbColor},
    prelude::*,
    text::{Alignment, Text},
};

use ili9488_rs::{Ili9488, Orientation, Rgb111, Rgb666Mode};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
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

    const FONT_WIDTH: usize = 20;
    const FONT_HEIGHT: usize = 36;

    let style = SevenSegmentStyleBuilder::new()
        .digit_size(Size::new(FONT_WIDTH as u32, FONT_HEIGHT as u32))
        .digit_spacing(6) // 6px spacing between digits
        .segment_width(5) // 5px wide segments
        .inactive_segment_color(Rgb666::CSS_DODGER_BLUE)
        .segment_color(Rgb666::BLACK)
        .build();

    // Render
    display.clear(Rgb666::CSS_DODGER_BLUE).unwrap();

    let mut i = 0u8;

    // Backend for the buffer
    const BUFFER_WIDTH: usize = (FONT_WIDTH + 6) * 3;
    let mut buffer_data = [Rgb666::CSS_DODGER_BLUE; BUFFER_WIDTH * FONT_HEIGHT];
    loop {
        // Setup Frame buffer
        let mut fbuf = FrameBuf::new(&mut buffer_data, BUFFER_WIDTH, FONT_HEIGHT);
        if i > 100 {
            i = 0;
            fbuf.clear(Rgb666::CSS_DODGER_BLUE).unwrap();
        }
        let area = Rectangle::new(
            display.bounding_box().center()
                - Point::new(BUFFER_WIDTH as i32 / 2, FONT_HEIGHT as i32 / 2),
            fbuf.size(),
        );

        // Render text in frame buffer
        let mut buffer = itoa::Buffer::new();
        let text = Text::with_alignment(
            buffer.format(i),
            Point::new(BUFFER_WIDTH as i32, FONT_HEIGHT as i32),
            style,
            Alignment::Right,
        );
        text.draw(&mut fbuf).unwrap();

        // Render frame buffer
        let start = Instant::now().as_millis();
        display.fill_contiguous(&area, buffer_data).unwrap();
        let end = Instant::now().as_millis();
        info!("text render time: {} ms", end - start);

        i += 1;
        Timer::after_millis(100).await;
    }
}
