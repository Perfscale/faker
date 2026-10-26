//! Template perfscale library (RFC 005): `greet(name)` and `token([key])`.
//!
//! Build: `cargo build --release --target wasm32-wasip2` — the artifact at
//! `target/wasm32-wasip2/release/hello.wasm` is referenced from YAML as
//! `libraries: - use: ./hello.wasm`.

use perfscale_library_sdk::{args, export_library, Ctx, Error, FunctionInfo, Library};

// On host builds `export_library!` is a no-op (the component export exists
// only for wasm32), so the type is "never constructed" outside tests.
#[allow(dead_code)]
#[derive(Default)]
struct Hello {
    greeting: Option<String>,
}

impl Library for Hello {
    fn functions(&self) -> Vec<FunctionInfo> {
        vec![
            FunctionInfo {
                name: "greet",
                description: "Greeting for a name",
                secret: false,
            },
            FunctionInfo {
                name: "token",
                description: "Random 16-char token; optional memo key reuses it within one message",
                secret: true,
            },
        ]
    }

    fn init(&mut self, config: serde_json::Value) -> Result<(), Error> {
        // The YAML `with:` block arrives here as JSON. Returning Err is fatal
        // to the run.
        self.greeting = config
            .get("greeting")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        Ok(())
    }

    fn call(
        &mut self,
        ctx: &mut Ctx,
        func: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<String, Error> {
        match func {
            "greet" => {
                let name = args::string(&args, 0, func)?;
                Ok(format!(
                    "{}, {name}!",
                    self.greeting.as_deref().unwrap_or("hello")
                ))
            }
            "token" => {
                // All randomness must come from ctx.prng() — the sandbox never
                // provides wasi:random, and the seeded PRNG keeps `seed:` runs
                // reproducible.
                let mint = |c: &mut Ctx| {
                    const ALPHABET: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                    Ok((0..16)
                        .map(|_| ALPHABET[c.prng().below(36) as usize] as char)
                        .collect::<String>())
                };
                // `${hello.token(order)}` appearing twice in one message
                // yields one token; `${hello.token()}` is always fresh.
                match args::optional_string(&args, 0) {
                    Some(key) => ctx.memo(&key, mint),
                    None => mint(ctx),
                }
            }
            // A known alias + unknown function fails the step — never silently
            // pass a typo into a payload.
            other => Err(Error::new(format!(
                "hello.{other}: unknown function — exports: greet, token"
            ))),
        }
    }
}

export_library!(Hello);

#[cfg(test)]
mod tests {
    use super::*;
    use perfscale_library_sdk::test_call;

    // The SDK harness: no wasm runtime, no engine — call the trait directly.
    #[test]
    fn greet_uses_default_and_configured_greeting() {
        let mut lib = Hello::default();
        let mut ctx = Ctx::new(42);
        ctx.message_seq = 1;
        assert_eq!(
            test_call(&mut lib, &mut ctx, "greet", vec!["world".into()]).unwrap(),
            "hello, world!"
        );

        let mut lib = Hello::default();
        lib.init(serde_json::json!({ "greeting": "hi" })).unwrap();
        assert_eq!(
            test_call(&mut lib, &mut ctx, "greet", vec!["world".into()]).unwrap(),
            "hi, world!"
        );
        assert!(test_call(&mut lib, &mut ctx, "greet", vec![]).is_err());
    }

    #[test]
    fn token_is_memoized_per_key_within_one_message() {
        let mut lib = Hello::default();
        let mut ctx = Ctx::new(42);
        ctx.message_seq = 1;
        let a = test_call(&mut lib, &mut ctx, "token", vec!["order".into()]).unwrap();
        assert_eq!(a.len(), 16);
        let b = test_call(&mut lib, &mut ctx, "token", vec!["order".into()]).unwrap();
        assert_eq!(a, b, "same message + key → same token");
        let other = test_call(&mut lib, &mut ctx, "token", vec!["line".into()]).unwrap();
        assert_ne!(a, other, "other key is independent");
        ctx.message_seq = 2;
        let c = test_call(&mut lib, &mut ctx, "token", vec!["order".into()]).unwrap();
        assert_ne!(a, c, "next message → fresh token");
        // No key → always fresh.
        let d = test_call(&mut lib, &mut ctx, "token", vec![]).unwrap();
        let e = test_call(&mut lib, &mut ctx, "token", vec![]).unwrap();
        assert_ne!(d, e);
    }

    #[test]
    fn same_seed_reproduces_tokens() {
        let mut lib = Hello::default();
        let mut ctx1 = Ctx::new(7);
        let mut ctx2 = Ctx::new(7);
        let a = test_call(&mut lib, &mut ctx1, "token", vec![]).unwrap();
        let b = test_call(&mut lib, &mut ctx2, "token", vec![]).unwrap();
        assert_eq!(a, b, "seeded PRNG is deterministic");
    }

    #[test]
    fn unknown_function_fails() {
        let mut lib = Hello::default();
        let mut ctx = Ctx::new(1);
        let err = test_call(&mut lib, &mut ctx, "nope", vec![]).unwrap_err();
        assert!(err.message().contains("unknown function"));
    }
}
