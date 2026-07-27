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
6. Public source of truth: https://github.com/ASAzhangyongchao/luozi. Do not recreate the GitHub repo or convert `apps/luozi` into a knowledge-system submodule without maintainer confirmation.
7. Signing: same interim policy as QuotaPet — ad-hoc first; no notarized Release until personal Developer ID is ready. Packaging/Release scripts can land later.
