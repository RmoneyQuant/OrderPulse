# Contributing to OrderPulse

## Development Setup

1. Create and activate a virtual environment.
2. Install build tooling and editable extension.

```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip maturin
maturin develop --release
```

## Running Tests

```bash
cargo test -q
source .venv/bin/activate
python -m unittest -v test_streaming
```

## Coding Guidelines

- Keep public APIs backward compatible unless doing a planned breaking release.
- Add tests for all bug fixes and new public behavior.
- Prefer clear, explicit errors over silent fallback behavior.
- Keep Rust and Python docs in sync.

## Pull Request Checklist

- [ ] Rust tests pass
- [ ] Python tests pass
- [ ] README/API docs updated if behavior changed
- [ ] Version bump is appropriate
- [ ] Changelog entry added
