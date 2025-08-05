#![no_std]
#![no_main]

use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::{Delay, Instant, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::MonoTextStyle,
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

    let style = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb666::WHITE);

    // Render
    display.clear_screen_fast(Rgb111::WHITE).unwrap();

    let mut prev = 0u8;
    let mut i = 0u8;
    loop {
        let mut buffer = itoa::Buffer::new();
        let mut text = Text::with_alignment(
            buffer.format(prev),
            display.bounding_box().center() + Point::new(0, 0),
            style,
            Alignment::Center,
        );
        text.draw(&mut display).unwrap();

        let start = Instant::now().as_millis();

        let mut buffer = itoa::Buffer::new();
        text.character_style.text_color = Some(Rgb666::BLACK);
        text.text = buffer.format(i);
        text.draw(&mut display).unwrap();

        let end = Instant::now().as_millis();
        info!("text render time: {} ms", end - start);

        prev = i;
        i += 1;
        if i > 100 {
            i = 0;
        }
        Timer::after_millis(500).await;
    }
}
