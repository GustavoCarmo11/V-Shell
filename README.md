# V-Shell rail compile fix

This patch fixes the two Rust errors shown by `amane dev` after the rail update.

## 1. Replace `src/special_workspace.rs`

Copy the included `src/special_workspace.rs` into `~/Projects/V-Shell/src/special_workspace.rs`.

The fix removes the call to `Service::quiet()`, which is `pub(crate)` inside Amane. It now checks the current state with `Self::read()` and returns before acquiring the write guard when an event does not change anything. A write notification is therefore emitted only for an actual state change.

## 2. Add `serde_json` to the manifest Amane actually builds

The error path shows that `amane dev` is compiling the generated project at `~/.cache/amane/project`, not using only the manifest at the V-Shell root. Add the direct dependency to that generated manifest:

```bash
cargo add serde_json@1.0.154 --manifest-path ~/.cache/amane/project/Cargo.toml
```

The V-Shell root `Cargo.toml` already has `serde_json = "1.0.154"` in the rail-update package, but the generated manifest used by `amane dev` needs its own direct dependency declaration.

If `cargo add` is unavailable, add this under `[dependencies]` in `~/.cache/amane/project/Cargo.toml`:

```toml
serde_json = "1.0.154"
```

Then run:

```bash
cd ~/Projects/V-Shell
cargo fmt
amane dev
```

The `dead_code` warning for `focus::set` is unrelated and does not stop compilation.
