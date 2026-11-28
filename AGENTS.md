# Repository Guidelines

## Project Overview and Technologies

V-Shell is a Caelestia-inspired Rust desktop shell built on Amane. It uses Wayland layer-shell, Vello/WGPU, GLSL and Hyprland IPC. Amane supplies desktop-app, media, D-Bus and audio services. The separate `color-worker` uses `image` and `material-color-utils` for wallpaper palettes. `cava` is the optional visualizer; `curl` fetches remote artwork.

## Structure and Architecture

`src/main.rs` composes per-monitor windows. `src/modules/dashboard/` contains Dashboard pages; `src/motion/` contains spring and deformation logic. Other `src/` modules implement the launcher, theme, wallpapers and system indicators. `shaders/shell-frame.frag` draws the shared frame. `color-worker/` is a standalone Rust binary. Motion tests are colocated in `src/motion/`.

UI composition uses Amane widgets. Put reactive background state in Amane `Service` implementations and keep it distinct from rendering. Hyprland events use Unix sockets.

## Build, Test, and Run

Amane generates ignored Cargo manifests with local paths. From the root, configure the target once if needed, then run:

```sh
amane target "$PWD"  # only if V-Shell is not already the configured target
amane dev             # build, run, and rebuild when Rust sources change
amane compile         # build without starting the shell
```

After Amane generates the root manifest, use `cargo fmt` and `cargo fmt --check`. Run unit tests with `cargo test --manifest-path Cargo.toml`; install the worker with `cargo install --path color-worker`. Root `Cargo.toml` and `Cargo.lock` are intentionally ignored; do not commit generated copies.

## Code Style and Naming

Follow `rustfmt` defaults. Use `snake_case` for functions, modules and files; `UpperCamelCase` for types and traits; and `UPPER_SNAKE_CASE` for constants. Keep changes within the relevant module and reuse shared motion, theme and Material tokens where applicable.

## Security and Change Practices

Never interpolate metadata, paths or URLs into shell commands; use `Command` arguments and timeouts. Treat wallpaper paths and MPRIS artwork URLs as inputs: validate protocols and limit downloads. Do not commit secrets or generated manifests. Preserve XDG paths and existing user data when changing caches or state.

Tests use Rust's built-in `#[test]` framework. Add focused tests beside the logic, particularly for motion math, parsers and state transitions. Existing tests cover motion; the worker has no dedicated tests. No coverage threshold is defined.

## Commits and Pull Requests

Recent commit subjects are concise, mostly Portuguese, sometimes using scopes such as `style(bar):`. Use an action-oriented subject. PRs should explain behavior, list commands run, include screenshots for visual changes, and note compositor or hardware requirements.
