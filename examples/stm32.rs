#![no_std]
#![no_main]

use defmt::*;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::{Delay, Timer};
use embedded_hal::spi::SpiDevice;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_8X13, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Alignment, Text},
};

use ili9488::{Ili9488, Ili9488PixelFormat, Orientation, Rgb111Mode};

fn bit(status: u32, pos: u8) -> u8 {
    if (status & (1 << pos)) > 0 {
        1
    } else {
        0
    }
}

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
    spi_config.frequency = Hertz::mhz(80);

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

    info!("Hello World");
    info!("Initializing Display...");

    let display = Ili9488::new(
        spi_interface,
        reset_pin,
        &mut delay,
        Orientation::LandscapeFlipped,
        Rgb111Mode,
    )
    .unwrap();
    info!("Done!");

    // let mut read_byte = [0u8; 2];
    // let mut read = [0u8; 3 * 10];
    let (spi, reset) = display.release();
    let (mut spi, mut dc) = spi.release();

    const ReadDisplayIdentificationInformation: u8 = 0x04;
    const ReadID1: u8 = 0xDA;
    const ReadID2: u8 = 0xDB;
    const ReadID3: u8 = 0xDC;
    const ReadDisplayStatus: u8 = 0x09;
    const MemoryRead: u8 = 0x2E;
    const ReadDisplayBrightness: u8 = 0x52;
    const ReadDisplayPixelFormat: u8 = 0x0C;

    let mut read = [0u8; 2];
    dc.set_low();

    spi.transfer(&mut read, &[ReadID1]).unwrap();
    info!("ID1 (LCD module’s manufacturer ID): {}", read);

    spi.transfer(&mut read, &[ReadID2]).unwrap();
    info!("ID2 (LCD module/driver version): {}", read);

    spi.transfer(&mut read, &[ReadID3]).unwrap();
    info!("ID3 (LCD module/driver): {}", read);

    spi.transfer(&mut read, &[ReadDisplayBrightness]).unwrap();
    info!("Brightness: {}", read);

    spi.transfer(&mut read, &[ReadDisplayPixelFormat]).unwrap();
    info!("Display Pixel Format: {}", read);
    info!("Display Pixel Format: {:b}", read[1]);

    let mut read = [0u8; 4];
    spi.transfer(&mut read, &[ReadDisplayStatus]).unwrap();
    let status: u32 = (u32::from(read[0]) << 24)
        | (u32::from(read[1]) << 16)
        | (u32::from(read[2]) << 8)
        | u32::from(read[3]);
    info!("Display Status: {:b}", read);
    info!("Display Status: {:b}", status);
    info!("Booster Voltage Status: {}", bit(status, 31));
    info!("Row Address Order: {}", bit(status, 30));
    info!("Column Address Order: {}", bit(status, 29));
    info!("Row/Column Exchange: {}", bit(status, 28));
    info!("Vertical Refresh: {}", bit(status, 27));
    info!("RGB/BGR Order: {}", bit(status, 26));
    info!("Horizontal Refresh Order: {}", bit(status, 25));
    info!("Pixel Format: {:b}", (status & (0b111u32 << 20)) >> 20);
    info!("Idle Mode On/Off: {}", bit(status, 19));
    info!("Partial Mode On/Off: {}", bit(status, 18));
    info!("Sleep In/Out: {}", bit(status, 17));
    info!("Display Normal Mode On/Off: {}", bit(status, 16));
    info!("Vertical Scrolling Status On/Off: {}", bit(status, 15));
    info!("Inversion Status On/Off: {}", bit(status, 13));
    info!("Display On/Off: {}", bit(status, 10));
    info!("Tearing Effect Line On/Off: {}", bit(status, 9));
    info!(
        "Gamma Curve Selection: {:b}",
        (status & (0b111u32 << 6)) >> 6
    );
    info!("Tearing Effect Line Mode: {}", bit(status, 5));

    let mut read = [0u8; 12];
    spi.transfer(&mut read, &[MemoryRead]).unwrap();
    info!("Memory: {}", read);

    dc.set_high();

    // spi.transfer(&mut read, &[0x2E]).unwrap();

    loop {
        // display.display_mode(ili9341::ModeState::On).unwrap();
        // Timer::after_millis(500).await;

        // display.display_mode(ili9341::ModeState::Off).unwrap();
        Timer::after_millis(500).await;
    }
}
