# Contributing

Thanks for contributing to Codineer.

## Development setup

- Install the stable Rust toolchain.
- Work from the repository root in this Rust workspace.

## Build

```bash
cargo build
cargo build --release
```

## Test and verify

Run the full Rust verification set before you open a pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo test --workspace
```

If you change behavior, add or update the relevant tests in the same pull request.

