# Vendored genawaiter (mmlx fork)

Upstream `whatisaphone/genawaiter` 0.99.1 is dormant; we vendor it to
control span hygiene in the `gen!`/`yield_!` transform (rust-analyzer
cannot see through the stock expansion) and to insulate the song
pipeline from a dead dependency.

- Sources: crates.io tarballs, sha256-verified against Cargo.lock.
- mmlx changes are marked with `// mmlx:` comments in the source.
- Do NOT re-vendor over this directory; rebase changes instead.
