# perfscale library template

The minimal starting point for authoring a **perfscale library** — a WASM
component that provides value-generating functions to `${alias.fn(...)}`
tokens in perfscale test payloads (RFC 005). Requires perfscale ≥ v0.21.0.

**Use this template:** click *Use this template → Create a new repository*
on GitHub (or `gh repo create my-lib --template Perfscale/library-template`),
then clone your new repo.

## What you get

`src/lib.rs` implements two functions demonstrating the two core concepts:

- `greet(name)` → `"hello, {name}!"` — a plain function
- `token([key])` → a random 16-char id from the SDK's seeded PRNG; the
  optional key memoizes per message (`${hello.token(order)}` twice in one
  message = one value; no key = always fresh)

plus runtime-free unit tests (`cargo test`) driven by the SDK's `test_call`
harness — no wasm runtime needed.

## Make it yours

1. **Rename the package** in `Cargo.toml` (`[package] name`). The crate name
   becomes the default token alias (`-` → `_`) and must match
   `[a-z][a-z0-9_]*`. Or keep the crate name and set `as: myalias` in YAML.
2. **Add a function:**
   - add a `FunctionInfo` entry in `functions()` (set `secret: true` for
     credential-minting functions — the engine masks their results in logs)
   - add a match arm in `call()`; use `args::string`/`int`/`float` to parse
     arguments, `ctx.prng()` for all randomness, `ctx.memo(&key, f)` for
     per-message reuse
3. **Configure it (optional):** override `init()` — the YAML `with:` block
   arrives there as JSON. Returning `Err` is fatal to the run.

## Build

```console
$ rustup target add wasm32-wasip2
$ cargo build --release --target wasm32-wasip2
```

(Rust ≥ 1.82 emits WASI Preview 2 components natively — no cargo-component.)

## Wire it into a perfscale test

```yaml
# config.yaml
vus: 1
duration: 10s
libraries:
  - use: ./target/wasm32-wasip2/release/hello.wasm  # path relative to this file
    with:                                           # optional, passed to init()
      greeting: hi
```

```yaml
# test.yaml — ${...} tokens expand in action payloads (http/ws/gRPC/GraphQL)
steps:
  - use: std/http@v1
    with:
      method: POST
      url: https://httpbin.org/post
      body:
        greeting: "${hello.greet(world)}"
        order_id: "${hello.token(order)}"
```

(`std/log@v1` resolves `${{ }}` step outputs, not `${...}` generator tokens.)

```console
$ cd examples && perfscale lint test.yaml config.yaml
$ perfscale run -f test.yaml -c config.yaml
```

See [`examples/`](examples/) for a runnable version (log + http steps).

## The rules that matter

- **Errors:** a known alias + unknown function (or bad arguments) **fails the
  step** — typos never slip silently into a payload. An unknown *alias* leaves
  the token verbatim.
- **Randomness:** draw everything from `ctx.prng()` (seeded per instance);
  `wasi:random` is never provided. Avoid `HashMap` (use `Vec`/`BTreeMap`) and
  `println!` (`wasi:cli/*` is never granted).
- **Capabilities:** `fs`/`clock` access must be granted in YAML
  (`capabilities: [clock]` under the `libraries:` entry, plus
  `allow_library_capabilities: true` in the config). Importing more than
  granted is a hard load error.
- **Determinism:** `seed:` in the config makes runs reproducible.

Full contract: [perfscale-library-sdk docs](https://github.com/Perfscale/perfscale/tree/v0.21.0/crates/perfscale-library-sdk).

## License

MIT OR Apache-2.0, same as perfscale.
