# Perfscale Faker

Deterministic Faker-style test data as a pure Rust/WASM
[Perfscale library](https://github.com/Perfscale/perfscale/tree/v0.28.0/docs/library-sdk.md).
It adds `internet`, `address`, and `payment` functions to `${faker...}` tokens
without changing the engine or granting filesystem, clock, random, or network
capabilities.

Version 0.1.0 deliberately provides a small embedded English corpus, not the
full Python Faker API. Tested with Perfscale v0.28.0.

## Install

Download the component and checksum from
[GitHub Releases](https://github.com/Perfscale/faker/releases), or build it
locally as shown below. Declare it in the test or config YAML; relative paths
resolve against the declaring file:

```yaml
libraries:
  - use: ./faker.wasm
    as: faker                    # optional: faker is the manifest default
    capabilities: []             # required; this component needs no grants
    with:
      language: en               # optional, default: en
      encoding: plain            # optional: plain (default) or url
```

Use the functions inside any payload that supports `${alias.fn(...)}` tokens:

```yaml
steps:
  - use: std/http@v1
    with:
      method: POST
      url: https://api.example.test/users
      body:
        email: "${faker.internet("email")}"
        city: "${faker.address("city")}"
```

A known function with an unknown kind, invalid arguments, or invalid `with:`
options fails the step instead of sending bad data. Calls return fresh values;
there is no optional memo-key argument in v0.1.0.

## Functions

| Call | Result |
|---|---|
| `internet("email")` | ASCII `first.lastN@example.com/net/org` address |
| `address("street_address")` | Street number and name |
| `address("zip_code")` | Five-digit postal code |
| `address("city")` | City |
| `address("state_abbr")` | Two-letter US state abbreviation |
| `address("country")` | Country |
| `payment("credit_card_number")` | Visa test number `4111111111111111` |
| `payment("credit_card_expiration_month")` | Month `1` through `12` |
| `payment("credit_card_expiration_year")` | Uniform year from current UTC year +1 through +71, inclusive |
| `payment("credit_card_cvv")` | Number `100` through `999` |

The email domains are reserved for examples. The component does not send
email, process payments, or fetch real personal data.

## Options

Both `with:` keys are optional. Unknown keys and values are errors.

| Option | Values | Default |
|---|---|---|
| `language` | `en` (the only corpus currently available) | `en` |
| `encoding` | `plain`, `url` | `plain` |

`encoding: url` percent-encodes the UTF-8 bytes of every returned value:
`@` becomes `%40`, spaces become `%20`, and URL-reserved punctuation is
escaped. This is useful when inserting generated values into an
`application/x-www-form-urlencoded` body or a URL query. Do not use this mode
for ordinary JSON values.

```yaml
libraries:
  - use: /loadgen/faker.wasm
    as: faker
    capabilities: []
    with: { language: en, encoding: url }

steps:
  - name: checkout
    use: std/http@v1
    with:
      method: POST
      url: "${{ env.BASE_URL }}/cart/checkout"
      headers: { Content-Type: application/x-www-form-urlencoded }
      body: 'email=${faker.internet("email")}&street_address=${faker.address("street_address")}&zip_code=${faker.address("zip_code")}&city=${faker.address("city")}&state=${faker.address("state_abbr")}&country=${faker.address("country")}&credit_card_number=${faker.payment("credit_card_number")}&credit_card_expiration_month=${faker.payment("credit_card_expiration_month")}&credit_card_expiration_year=${faker.payment("credit_card_expiration_year")}&credit_card_cvv=${faker.payment("credit_card_cvv")}'
    check: { status: 200 }
```

The [Online Boutique migration PR](https://github.com/GoogleCloudPlatform/microservices-demo/pull/3588)
uses the same component in its Docker-based load generator.

## Determinism and time

All random choices come from the Perfscale SDK's seeded PRNG. A config such as
`seed: 42` replays the same sequence for the same VUs, calls, and inputs.
Expiration years also depend on `ctx.time_ms`, so their range advances with
the UTC year. No WASI clock or random import is used.

## Docker distribution

Each release publishes `faker.wasm`, `faker.wasm.sha256`, and an artifact-only
OCI image. The image contains no executable runtime; it exists for `COPY
--from` in another Dockerfile:

```dockerfile
FROM ghcr.io/perfscale/faker:0.1.0 AS faker
FROM ghcr.io/perfscale/perfscale:0.28.0
WORKDIR /loadgen
COPY --from=faker /faker.wasm ./
COPY test.yaml config.yaml ./
```

Pin the GHCR multi-arch digest in production builds. No Rust toolchain or
source checkout is needed in the consuming image.

## Build and test

Rust 1.82 or newer emits WASI Preview 2 components directly:

```console
rustup target add wasm32-wasip2
cargo test
cargo build --release --target wasm32-wasip2
# target/wasm32-wasip2/release/faker.wasm
```

Run the real engine integration test without any external HTTP service:

```console
(cd e2e && perfscale lint faker.test.yaml config.yaml)
perfscale run -f e2e/faker.test.yaml -c e2e/config.yaml
```

On macOS, select the Command Line Tools SDK if the wasm build requires it:

```console
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
```

Documentation: [Perfscale/docs](https://github.com/Perfscale/docs/blob/main/en/concepts/faker.mdx).

## License

MIT OR Apache-2.0, same as Perfscale.
