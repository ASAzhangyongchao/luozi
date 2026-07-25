# Contributing

1. Keep product code in this repository (`apps/luozi` as an independent `.git`).
2. Do not commit models, secrets, or build artifacts.
3. Prefer small commits with clear intent.
4. Run before pushing a PR:

```bash
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

5. Windows CI compile success is not a substitute for real-machine hotkey/focus tests.
6. Do not create the public GitHub repository or knowledge-system submodule without maintainer confirmation.
