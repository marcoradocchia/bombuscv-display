<div align="center">
  <h1 align="center">BombusCV</h1>

  ![GitHub releases](https://img.shields.io/github/downloads/marcoradocchia/bombuscv-display/total?color=%23a9b665&logo=github)
  ![GitHub source size](https://img.shields.io/github/languages/code-size/marcoradocchia/bombuscv-display?color=ea6962&logo=github)
  ![GitHub open issues](https://img.shields.io/github/issues-raw/marcoradocchia/bombuscv-display?color=%23d8a657&logo=github)
  ![GitHub open pull requests](https://img.shields.io/github/issues-pr-raw/marcoradocchia/bombuscv-display?color=%2389b482&logo=github)
  ![GitHub sponsors](https://img.shields.io/github/sponsors/marcoradocchia?color=%23d3869b&logo=github)
  ![GitHub license](https://img.shields.io/github/license/marcoradocchia/bombuscv-display?color=%23e78a4e)
  <!-- ![Crates.io downloads](https://img.shields.io/crates/d/bombuscv-display?label=crates.io%20downloads&color=%23a9b665&logo=rust) -->
  <!-- ![Crates.io version](https://img.shields.io/crates/v/bombuscv-display?logo=rust&color=%23d8a657) -->
</div>

I2C (SSD1306) display integration for `bombuscv-rs`.

## Index

- [Use case](#use-case)
- [Examples](#examples)
- [Install](#install)
  - [Cargo](#cargo)
    - [Master branch](#master-branch)
- [Usage](#usage)
- [Changelog](#changelog)
- [License](#license)

## Use case

This software is intended to **extend & enhance**
[`bobmuscv-rs`](https://github.com/marcoradocchia/bombuscv-rs) functionality.

`bombuscv-display` is built to display and immediatly visualize `bombuscv-rs`
status, system and humidity/temperature sensor (i.e. DHT22) information on a
**SSD1306** oled display[^1] powered by *Raspberry Pi* GPIO.

[^1]: 0.96", 128x64 pixels, I2C display

## Examples

<!-- TODO -->

## Install

### Cargo

In order to install using Rust

#### Master branch

To build and install from master branch run:
```sh
cargo install --git https://github.com/marcoradocchia/bombuscv-display --branch master
```

## Usage

`bombuscv-display` reads *humidity* and *temperature* data from standard input
(i.e. a _unix pipeline_), so it's strongly recommended to use it in combination
with [`datalogger`](https://github.com/marcoradocchia/datalogger) or any other
software which is able to print to standard output such data in the following
*csv like* format: `<humidity>,<temperature>`.

<!-- TODO -->
<!-- ```sh -->
<!-- TODO! -->
<!-- ``` -->

## Changelog

Complete [CHANGELOG](CHANGELOG.md).

## License

[GPLv3](LICENSE)
