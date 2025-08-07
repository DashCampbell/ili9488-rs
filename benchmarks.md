
# Benchmarking Results

Benchmarking results for different [optimization levels](https://doc.rust-lang.org/rustc/codegen-options/index.html#opt-level).

## benchmarking.rs

Benchmarking the time it takes to fully clear the screen with a constant color using different pixel formats.

> opt-level = 3

binary-size = 24 kb

pixel-format (rgb 6-6-6): 443 ms 

pixel-format (rgb 1-1-1): 175 ms 

pixel-format (rgb 1-1-1) fast version: 52 ms

> opt-level = s

binary-size = 19 kb

pixel-format (rgb 6-6-6): 635 ms 

pixel-format (rgb 1-1-1): 255 ms 

pixel-format (rgb 1-1-1) fast version: 59 ms 

> opt-level = z

binary-size = 17 kb

pixel-format (rgb 6-6-6): 1110 ms 

pixel-format (rgb 1-1-1): 390 ms 

pixel-format (rgb 1-1-1) fast version: 108 ms 

## nyan_cat.rs

> opt-level = 3

binary-size = 40 kb

Total Rendering Time: 19ms

FPS: 52 

> opt-level = s

binary-size = 32 kb

Total Rendering Time: 28ms

FPS: 36

> opt-level = 3

binary-size = 30 kb

Total Rendering Time: 47ms

FPS: 21
