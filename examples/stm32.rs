#![no_std]
#![no_main]

use defmt::*;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::{self, Mode, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::{Delay, Timer};
use embedded_graphics::mono_font::iso_8859_14::FONT_10X20;
use embedded_graphics::pixelcolor::raw::ToBytes;
use embedded_hal::spi::SpiDevice;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_8X13, MonoTextStyle},
    pixelcolor::{Bgr666, Rgb666, RgbColor},
    prelude::*,
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
    text::{Alignment, Text},
};

use ili9488_rs::{Ili9488, Ili9488PixelFormat, Orientation, Rgb111, Rgb111Mode, Rgb666Mode};

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

    info!("Hello World");
    info!("Initializing Display...");

    let mut display = Ili9488::new(
        spi_interface,
        reset_pin,
        &mut delay,
        Orientation::LandscapeFlipped,
        Rgb666Mode,
    )
    .unwrap();
    info!("Done!");
    let color = Rgb111::RED;
    info!("Color: {:08b}", color.into_storage());
    let color = (color.into_storage() << 3) | color.into_storage();
    info!("Color: {:08b}", color);

    info!("Clearing Screen...");
    display.clear_screen(Rgb666::GREEN).unwrap();
    info!("Done");

    info!("Clearing Screen Again...");
    display.clear_screen_fast(Rgb111::WHITE).unwrap();
    info!("Done");

    // Embedded graphics stuff
    let thin_stroke = PrimitiveStyle::with_stroke(Rgb666::BLACK, 1);
    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb666::BLACK);
    let text = "embedded-graphics";
    Text::with_alignment(
        text,
        display.bounding_box().center() + Point::new(0, 15),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

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
    info!("Display Pixel Format: {:08b}", read[1]);

    let mut read = [0u8; 4];
    spi.transfer(&mut read, &[ReadDisplayStatus]).unwrap();
    let status: u32 = (u32::from(read[0]) << 24)
        | (u32::from(read[1]) << 16)
        | (u32::from(read[2]) << 8)
        | u32::from(read[3]);
    info!("Display Status: {:b}", read);
    info!("Display Status: {:032b}", status);
    info!("Booster Voltage Status: {}", bit(status, 31));
    info!("Row Address Order: {}", bit(status, 30));
    info!("Column Address Order: {}", bit(status, 29));
    info!("Row/Column Exchange: {}", bit(status, 28));
    info!("Vertical Refresh: {}", bit(status, 27));
    info!("RGB/BGR Order: {}", bit(status, 26));
    info!("Horizontal Refresh Order: {}", bit(status, 25));
    info!("Pixel Format: {:03b}", (status & (0b111u32 << 20)) >> 20);
    info!("Idle Mode On/Off: {}", bit(status, 19));
    info!("Partial Mode On/Off: {}", bit(status, 18));
    info!("Sleep In/Out: {}", bit(status, 17));
    info!("Display Normal Mode On/Off: {}", bit(status, 16));
    info!("Vertical Scrolling Status On/Off: {}", bit(status, 15));
    info!("Inversion Status On/Off: {}", bit(status, 13));
    info!("Display On/Off: {}", bit(status, 10));
    info!("Tearing Effect Line On/Off: {}", bit(status, 9));
    info!(
        "Gamma Curve Selection: {:03b}",
        (status & (0b111u32 << 6)) >> 6
    );
    info!("Tearing Effect Line Mode: {}", bit(status, 5));

    let mut read = [0u8; 12];
    spi.transfer(&mut read, &[MemoryRead]).unwrap();
    info!("Memory: {}", read);

    let col = Rgb666::WHITE.into_storage() << 2;
    let a = col.to_be_bytes();
    let b = col.to_le_bytes();
    let c = col.to_ne_bytes();
    // let d = col.into_storage();
    let d = col;
    // info!(
    //     "Rgb666: r={:08b}, g={:08b}, b={:08b}",
    //     col.r(),
    //     col.g(),
    //     col.b()
    // );
    info!("Rgb666: Red storage= {:#032b}", d);
    info!(
        "Rgb666: Red be= {:#010b} {:#010b} {:#010b}",
        a[0], a[1], a[2]
    );
    info!(
        "Rgb666: Red le= {:#010b} {:#010b} {:#010b}",
        b[0], b[1], b[2]
    );
    info!(
        "Rgb666: Red le= {:#010b} {:#010b} {:#010b}",
        c[0], c[1], c[2]
    );

    dc.set_high();

    // spi.transfer(&mut read, &[0x2E]).unwrap();

    loop {}
}
