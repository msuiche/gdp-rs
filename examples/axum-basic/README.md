# axum-basic

The gdp-ts `express-basic` example, ported to [axum](https://github.com/tokio-rs/axum).

- `src/proofs/`: one trusted module per fact. The only place each proof can be minted.
- `src/data.rs`: the data layer. Every sensitive function demands a proof, so it is safe to call from anywhere.
- `src/lib.rs`: handlers. `Viewer` only authenticates; each handler names its inputs with `gdp::name2_async`, proves what it needs, and turns `None` into a 403 or 402.

```bash
cargo test                  # HTTP-level tests with tower's oneshot
cargo run                   # serves on 127.0.0.1:3000
curl -X PUT -H 'x-user: alice' -H 'content-type: application/json' \
     -d '{"password":"pw"}' localhost:3000/projects/acme/password-protection   # 204
curl -H 'x-user: bob' localhost:3000/projects/acme/password-protection         # {"enabled":true,"grantedBy":"UserHasProjectAccess"}
```

Authentication is an `x-user` header to keep the example small; use a real session in your app.
