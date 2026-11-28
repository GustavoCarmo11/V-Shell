# Repository Guidelines

## Project Overview and Technologies

V-Shell is a Caelestia-inspired Rust Wayland shell built with Amane, Vello/WGPU and GLSL; it integrates with Hyprland IPC. Amane supplies desktop, media, D-Bus and audio services. `color-worker/` uses `image` and `material-color-utils` for wallpaper palettes. `cava` optionally visualizes audio; `curl` fetches remote artwork.

## Structure and Architecture

`src/main.rs` composes per-monitor windows. `src/modules/dashboard/` contains Dashboard pages; `src/motion/` contains spring and deformation logic. Other `src/` modules implement launcher, theme, wallpapers and system indicators. `shaders/shell-frame.frag` draws the frame; `color-worker/` is a separate binary. Motion tests live beside their code in `src/motion/`.

Use Amane widgets for UI and `Service` implementations for reactive state. Keep state updates distinct from rendering. Hyprland events use Unix sockets.

## Amane Fork

The framework fork is [GustavoCarmo11/amane](https://github.com/GustavoCarmo11/amane).

## Build, Test, and Run

Amane generates ignored Cargo manifests with local paths. From the repository root, configure the target if needed, then run:

```sh
amane target "$PWD"  # only if V-Shell is not already the configured target
amane dev             # build, run, and rebuild when Rust sources change
amane compile         # build without starting the shell
```

After Amane generates the root manifest, run `cargo fmt`, `cargo fmt --check`, or `cargo test --manifest-path Cargo.toml`. Install the worker with `cargo install --path color-worker`. Do not commit ignored root manifests.

## Code Style and Naming

Follow `rustfmt` defaults. Use `snake_case` for functions, modules and files; `UpperCamelCase` for types and traits; and `UPPER_SNAKE_CASE` for constants. Keep changes within the relevant module and reuse shared motion, theme and Material tokens where applicable.

## Security and Change Practices

Pass external values as `Command` arguments and set timeouts. Validate wallpaper paths and MPRIS URLs; cap downloads. Do not commit secrets or generated manifests, and preserve XDG paths and user data.

Tests use Rust's built-in `#[test]` framework. Add focused tests beside the logic, particularly for motion math, parsers and state transitions. Existing tests cover motion; the worker has no dedicated tests. No coverage threshold is defined.

## Commits and Pull Requests

Recent commit subjects are concise, mostly Portuguese, sometimes scoped (e.g. `style(bar):`). Use action-oriented subjects. PRs should explain behavior, list commands run, include screenshots for visual changes, and note compositor or hardware requirements.
