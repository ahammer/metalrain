# Demo Launcher

Unified command-line tool to run any of the demo crates in this workspace without remembering individual crate paths.

## Usage

Run with a demo name (direct launch):

```bash
cargo run -p demo_launcher -- <demo-name>
```

List available demos:

```bash
cargo run -p demo_launcher -- --list
```

If you run it without a demo name, it now presents an interactive numbered menu:

```bash
cargo run -p demo_launcher
# Shows a list like:
#  [1] architecture_test    - Minimal architecture integration demo
#  [2] compositor_test      - Compositor + rendering layers stress test
#  [3] metaballs_test       - Metaball renderer clustering / presentation demo
#  [4] physics_playground   - Interactive physics playground
# Then waits for you to type a number (or 'q' to quit).
```

## Supported Demos

Names match each demo crate's exported `DEMO_NAME` constant:

- architecture_test
- compositor_test
- metaballs_test
- physics_playground

## Examples

```bash
# Launch physics playground
cargo run -p demo_launcher -- physics_playground

# Launch compositor test
cargo run -p demo_launcher -- compositor_test

# See list
cargo run -p demo_launcher -- --list
```

## Future Enhancements

- Interactive fuzzy picker when no argument supplied
- Watch mode to re-run last demo after changes
- Optional feature flags to reduce build times by excluding demos
