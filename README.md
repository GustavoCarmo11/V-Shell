# G-Shell

A lightweight Linux desktop shell written in Rust, built with [Amane](https://github.com/GustavoCarmo11/amane).

Designed for Wayland and currently tested with Hyprland.

## Features

- Desktop bar with workspace indicators and switching.
- Wallpaper transitions with optimized images.
- Dynamic Material You color palettes extracted from wallpapers.
- Light and dark theme support.
- Launcher and session controls.
- Per-wallpaper palette cache with limited disk usage.

## Requirements

- Linux with Wayland.
- Hyprland.
- Rust and Cargo.
- Rubik font.
- Material Symbols Rounded font.

## Installation

### 1. Install the patched Amane

G-Shell uses a fork of Amane with a fix for Hyprland special workspaces.

```bash
git clone https://github.com/GustavoCarmo11/amane.git ~/Projects/amane
cd ~/Projects/amane

./install.sh
cargo install --path cli --force
```

Make sure `~/.cargo/bin` is in your PATH.

### 2. Clone G-Shell

```bash
git clone https://github.com/GustavoCarmo11/G-Shell.git ~/Projects/G-Shell
cd ~/Projects/G-Shell
```

Configure the Amane target:

```bash
amane target "$PWD"
```

This target is local to the machine and must be configured after cloning.

### 3. Install the color worker

From the G-Shell directory:

```bash
cargo install --path color-worker --force
```

The default worker location is:

```text
~/.cargo/bin/caelestia-color-worker
```

To use another location:

```bash
export CAELESTIA_RS_COLOR_WORKER="/path/to/caelestia-color-worker"
```

### 4. Configure wallpapers

By default, wallpapers are loaded from:

```text
~/Pictures/Wallpapers
```

Supported formats:

- PNG
- JPG / JPEG
- WebP

To use another directory:

```bash
export CAELESTIA_RS_WALLPAPER_DIR="$HOME/Pictures/Wallpapers"
```

### 5. Fonts

Install these font families:

- Rubik
- Material Symbols Rounded

Refresh the font cache if necessary:

```bash
fc-cache -f
```

### 6. Run G-Shell

Development mode:

```bash
amane dev
```

Optimized build:

```bash
amane compile
```

Run the compiled shell:

```bash
amane run
```

## Updating

Update Amane:

```bash
cd ~/Projects/amane
git pull --ff-only
cargo install --path cli --force
```

Update G-Shell:

```bash
cd ~/Projects/G-Shell
git pull --ff-only
cargo install --path color-worker --force
```

## Project structure

```text
G-Shell/
├── src/
│   ├── main.rs
│   ├── theme.rs
│   └── wallpaper.rs
├── color-worker/
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   └── Cargo.lock
└── README.md
```

The root Cargo manifest is generated locally by Amane and is not tracked because it contains machine-specific paths.

## Related projects

- [G-Shell](https://github.com/GustavoCarmo11/G-Shell)
- [G-Shell Amane fork](https://github.com/GustavoCarmo11/amane)
- [Original Amane project](https://github.com/MystiaFin/amane)
