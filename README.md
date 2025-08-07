# `ili9488-rs`

[![Crates.io](https://img.shields.io/crates/d/ili9488-rs.svg)](https://crates.io/crates/ili9488-rs)
[![Crates.io](https://img.shields.io/crates/v/ili9488-rs.svg)](https://crates.io/crates/ili9488-rs)
[![Released API docs](https://docs.rs/ili9488-rs/badge.svg)](https://docs.rs/ili9488-rs)

> A platform agnostic driver to interface with the ILI9488 TFT
> LCD display

> This is a fork of [ili9431-rs](https://github.com/yuri91/ili9341-rs) that was coverted to an ILI9488 driver

> For an alternative ILI9488 driver, checkout [mipidsi](https://github.com/almindor/mipidsi)  

## What works

- Putting pixels on the screen
- Change the screen orientation
- Hardware scrolling
- Compatible with [embedded-graphics](https://docs.rs/embedded-graphics)

## TODO

- [ ] Add Rgb111 for embedded-graphics
- [ ] Add touchscreen example
- [ ] ???

## Examples

Run examples using
```bash
cargo run --example <example name>  --release

# example:
cargo run --example hello_world  --release
```

Examples are configured for an STM32L432KC.

See the [Display Data Format](https://www.displayfuture.com/Display/datasheet/controller/ILI9488.pdf#page=119) section of the ILI9488's datasheet for allowed pixel formats.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

### Resources

> ILI9488 Datasheet: https://www.hpinfotech.ro/ILI9488.pdf

> Lcd Wiki: https://www.lcdwiki.com/3.5inch_SPI_Module_ILI9488_SKU:MSP3520

### Credits

This crate is a fork of https://github.com/yuri91/ili9341-rs.

Bodmer's [TFT_eSPI](https://github.com/Bodmer/TFT_eSPI/blob/master/TFT_Drivers/ILI9488_Init.h) library for the ILI9488's initialization sequence.

Nyan Cat animation frames: https://github.com/iliana/html5nyancat/tree/master