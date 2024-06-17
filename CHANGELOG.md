# Changelog

## Rust v0.2.0

This release focuses on simplifying the public interface and removes some
non-idiomatic enumerations and methods. It also introduces two more color
spaces, Rec. 2020 and its linear variant.

### Added

- `ColorSpace::Rec2020` and `ColorSpace::LinearRec2020`
- `Color::lighten` and `Color::darken`
- `Color::to_24_bit`

### Changed

- `DefaultColor` is now `Layer`

### Removed

- `Coordinate`: use `usize` in index expressions instead
- `EightBitColor::is_ansi`, `EightBitColor::ansi`, `EightBitColor::is_rgb`,
  `EightBitColor::rgb`, `EightBitColor:is_gray`, `EightBitColor::gray`: use
  `match` instead
