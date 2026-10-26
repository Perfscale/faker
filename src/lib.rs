#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

//! Deterministic Faker-style data for Perfscale payloads.
//!
//! Build with `cargo build --release --target wasm32-wasip2` and declare
//! `target/wasm32-wasip2/release/faker.wasm` in a `libraries:` block.

use perfscale_library_sdk::{args, export_library, Ctx, Error, FunctionInfo, Library, Prng};
use serde_json::Value;

const FUNCTIONS: [FunctionInfo; 3] = [
    FunctionInfo {
        name: "internet",
        description: "Internet data: email",
        secret: false,
    },
    FunctionInfo {
        name: "address",
        description: "Address data: street_address, zip_code, city, state_abbr, country",
        secret: false,
    },
    FunctionInfo {
        name: "payment",
        description: "Test payment data: credit_card_number, expiration month/year, CVV",
        secret: false,
    },
];

const FIRST_NAMES: &[&str] = &[
    "Alex", "Avery", "Casey", "Emma", "Ethan", "Grace", "Henry", "Lena", "Liam", "Maya", "Noah",
    "Olivia", "Owen", "Ruby", "Sofia", "Theo",
];
const LAST_NAMES: &[&str] = &[
    "Anderson", "Baker", "Bennett", "Brooks", "Carter", "Clark", "Davis", "Ellis", "Foster",
    "Gray", "Hayes", "Kelly", "Miller", "Morgan", "Parker", "Reed",
];
const EMAIL_DOMAINS: &[&str] = &["example.com", "example.net", "example.org"];
const STREETS: &[&str] = &[
    "Main Street",
    "Maple Avenue",
    "Oak Street",
    "Cedar Road",
    "Market Street",
    "Lake View Drive",
    "Hillcrest Avenue",
    "Sunset Boulevard",
];
const CITIES: &[&str] = &[
    "Mountain View",
    "Springfield",
    "Riverton",
    "Fairview",
    "Georgetown",
    "Franklin",
    "Clinton",
    "Greenville",
    "Bristol",
    "Dover",
];
const STATES: &[&str] = &["CA", "NY", "TX", "WA", "OR", "NV", "CO", "IL", "MA", "FL"];
const COUNTRIES: &[&str] = &[
    "USA",
    "Canada",
    "United Kingdom",
    "Germany",
    "France",
    "Japan",
    "Australia",
    "Brazil",
];
const ADDRESS_KINDS: &[&str] = &[
    "street_address",
    "zip_code",
    "city",
    "state_abbr",
    "country",
];
const PAYMENT_KINDS: &[&str] = &[
    "credit_card_number",
    "credit_card_expiration_month",
    "credit_card_expiration_year",
    "credit_card_cvv",
];

#[derive(Clone, Copy, Default)]
enum Encoding {
    #[default]
    Plain,
    Url,
}

impl Encoding {
    fn apply(self, value: String) -> String {
        match self {
            Self::Plain => value,
            Self::Url => percent_encode(&value),
        }
    }
}

#[derive(Default)]
struct Faker {
    encoding: Encoding,
}

impl Library for Faker {
    fn functions(&self) -> Vec<FunctionInfo> {
        FUNCTIONS.to_vec()
    }

    fn pure(&self) -> bool {
        true
    }

    fn init(&mut self, config: Value) -> Result<(), Error> {
        let options = match config {
            Value::Null => return Ok(()),
            Value::Object(options) => options,
            _ => return Err(Error::new("faker: `with:` must be an object")),
        };
        for key in options.keys() {
            if !matches!(key.as_str(), "language" | "encoding") {
                return Err(Error::new(format!(
                    "faker: unknown option '{key}' — expected language or encoding"
                )));
            }
        }
        match options.get("language") {
            None => {}
            Some(Value::String(language)) if language == "en" => {}
            _ => {
                return Err(Error::new(
                    "faker.language: unsupported language — only 'en' is available",
                ))
            }
        }
        self.encoding = match options.get("encoding") {
            None => Encoding::Plain,
            Some(Value::String(encoding)) if encoding == "plain" => Encoding::Plain,
            Some(Value::String(encoding)) if encoding == "url" => Encoding::Url,
            _ => return Err(Error::new("faker.encoding: expected 'plain' or 'url'")),
        };
        Ok(())
    }

    fn call(&mut self, ctx: &mut Ctx, func: &str, args: Vec<Value>) -> Result<String, Error> {
        if args.len() != 1 {
            return Err(Error::new(format!("faker.{func}: expected {func}(kind)")));
        }
        let kind = args::string(&args, 0, func)?;
        let value = match func {
            "internet" => internet(ctx, &kind)?,
            "address" => address(ctx, &kind)?,
            "payment" => payment(ctx, &kind)?,
            other => {
                return Err(Error::new(format!(
                    "faker.{other}: unknown function — exports: internet, address, payment"
                )))
            }
        };
        Ok(self.encoding.apply(value))
    }
}

export_library!(Faker);

fn internet(ctx: &mut Ctx, kind: &str) -> Result<String, Error> {
    match kind {
        "email" => {
            let first = pick(ctx.prng(), FIRST_NAMES).to_ascii_lowercase();
            let last = pick(ctx.prng(), LAST_NAMES).to_ascii_lowercase();
            let suffix = ctx.prng().range(1, 99);
            let domain = pick(ctx.prng(), EMAIL_DOMAINS);
            Ok(format!("{first}.{last}{suffix}@{domain}"))
        }
        _ => Err(unknown_kind("internet", kind, &["email"])),
    }
}

fn address(ctx: &mut Ctx, kind: &str) -> Result<String, Error> {
    match kind {
        "street_address" => {
            let number = ctx.prng().range(1, 9999);
            let street = pick(ctx.prng(), STREETS);
            Ok(format!("{number} {street}"))
        }
        "zip_code" => Ok(ctx.prng().range(10000, 99999).to_string()),
        "city" => Ok(pick(ctx.prng(), CITIES).to_string()),
        "state_abbr" => Ok(pick(ctx.prng(), STATES).to_string()),
        "country" => Ok(pick(ctx.prng(), COUNTRIES).to_string()),
        _ => Err(unknown_kind("address", kind, ADDRESS_KINDS)),
    }
}

fn payment(ctx: &mut Ctx, kind: &str) -> Result<String, Error> {
    match kind {
        "credit_card_number" => Ok("4111111111111111".to_string()),
        "credit_card_expiration_month" => Ok(ctx.prng().range(1, 12).to_string()),
        "credit_card_expiration_year" => {
            let year = year_from_unix_ms(ctx.time_ms);
            Ok(ctx.prng().range(year + 1, year + 71).to_string())
        }
        "credit_card_cvv" => Ok(ctx.prng().range(100, 999).to_string()),
        _ => Err(unknown_kind("payment", kind, PAYMENT_KINDS)),
    }
}

fn pick<'a>(prng: &mut Prng, values: &'a [&str]) -> &'a str {
    values[prng.below(values.len() as u64) as usize]
}

fn unknown_kind(func: &str, kind: &str, kinds: &[&str]) -> Error {
    Error::new(format!(
        "faker.{func}: unknown kind '{kind}' — expected one of: {}",
        kinds.join(", ")
    ))
}

fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    out
}

fn year_from_unix_ms(ms: u64) -> i64 {
    let days = ((ms / 1000) as i64).div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year + 1
    } else {
        year
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perfscale_library_sdk::test_call;
    use serde_json::json;

    fn call(lib: &mut Faker, ctx: &mut Ctx, func: &str, kind: &str) -> String {
        test_call(lib, ctx, func, vec![kind.into()]).unwrap()
    }

    fn configured(encoding: &str) -> Faker {
        let mut lib = Faker::default();
        lib.init(json!({ "language": "en", "encoding": encoding }))
            .unwrap();
        lib
    }

    fn context(seed: u64) -> Ctx {
        let mut ctx = Ctx::new(seed);
        ctx.time_ms = 1_704_067_200_000;
        ctx
    }

    #[test]
    fn email_is_ascii_on_a_reserved_domain_and_seeded() {
        let mut left = Faker::default();
        let mut right = Faker::default();
        let email = call(&mut left, &mut context(42), "internet", "email");
        let replay = call(&mut right, &mut context(42), "internet", "email");
        assert_eq!(email, replay);
        let (local, domain) = email.split_once('@').unwrap();
        assert!(EMAIL_DOMAINS.contains(&domain));
        assert!(local.contains('.'));
        assert!(email.is_ascii());
        assert!(local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.'));
    }

    #[test]
    fn different_seeds_change_the_sequence() {
        let mut lib = Faker::default();
        let first = call(&mut lib, &mut context(1), "internet", "email");
        let second = call(&mut lib, &mut context(2), "internet", "email");
        assert_ne!(first, second);
    }

    #[test]
    fn address_values_have_the_expected_shape() {
        let mut lib = Faker::default();
        let mut ctx = context(7);
        let street = call(&mut lib, &mut ctx, "address", "street_address");
        let (number, name) = street.split_once(' ').unwrap();
        assert!((1..=9999).contains(&number.parse::<u32>().unwrap()));
        assert!(STREETS.contains(&name));
        let zip = call(&mut lib, &mut ctx, "address", "zip_code");
        assert_eq!(zip.len(), 5);
        assert!((10000..=99999).contains(&zip.parse::<u32>().unwrap()));
        assert!(CITIES.contains(&call(&mut lib, &mut ctx, "address", "city").as_str()));
        assert!(STATES.contains(&call(&mut lib, &mut ctx, "address", "state_abbr").as_str()));
        assert!(COUNTRIES.contains(&call(&mut lib, &mut ctx, "address", "country").as_str()));
    }

    #[test]
    fn payment_values_are_safe_and_in_range() {
        let mut lib = Faker::default();
        let mut ctx = context(11);
        assert_eq!(
            call(&mut lib, &mut ctx, "payment", "credit_card_number"),
            "4111111111111111"
        );
        for _ in 0..100 {
            let month = call(
                &mut lib,
                &mut ctx,
                "payment",
                "credit_card_expiration_month",
            );
            assert!((1..=12).contains(&month.parse::<u32>().unwrap()));
            let year = call(&mut lib, &mut ctx, "payment", "credit_card_expiration_year");
            assert!((2025..=2095).contains(&year.parse::<i64>().unwrap()));
            let cvv = call(&mut lib, &mut ctx, "payment", "credit_card_cvv");
            assert!((100..=999).contains(&cvv.parse::<u32>().unwrap()));
        }
    }

    #[test]
    fn url_mode_encodes_values_for_forms() {
        let mut lib = configured("url");
        let email = call(&mut lib, &mut context(42), "internet", "email");
        assert!(!email.contains('@'));
        assert!(email.contains("%40example."));
        let street = call(&mut lib, &mut context(7), "address", "street_address");
        assert!(!street.contains(' '));
        assert!(street.contains("%20"));
        assert_eq!(percent_encode("Mountain View&CA"), "Mountain%20View%26CA");
        assert_eq!(percent_encode("a+b@c"), "a%2Bb%40c");
        assert_eq!(percent_encode("é"), "%C3%A9");
    }

    #[test]
    fn options_default_to_english_and_plain_text() {
        let mut lib = Faker::default();
        lib.init(Value::Null).unwrap();
        let email = call(&mut lib, &mut context(42), "internet", "email");
        assert!(email.contains('@'));
        lib.init(json!({ "language": "en" })).unwrap();
        lib.init(json!({ "encoding": "plain" })).unwrap();
    }

    #[test]
    fn invalid_options_are_rejected() {
        let mut lib = Faker::default();
        assert!(lib
            .init(json!("en"))
            .unwrap_err()
            .message()
            .contains("object"));
        assert!(lib
            .init(json!({ "language": "ru" }))
            .unwrap_err()
            .message()
            .contains("only 'en'"));
        assert!(lib
            .init(json!({ "encoding": "xml" }))
            .unwrap_err()
            .message()
            .contains("plain"));
        assert!(lib
            .init(json!({ "locale": "en" }))
            .unwrap_err()
            .message()
            .contains("unknown option"));
    }

    #[test]
    fn unknown_functions_kinds_and_arity_are_errors() {
        let mut lib = Faker::default();
        let mut ctx = context(1);
        assert!(
            test_call(&mut lib, &mut ctx, "internet", vec!["username".into()])
                .unwrap_err()
                .message()
                .contains("email")
        );
        assert!(
            test_call(&mut lib, &mut ctx, "address", vec!["state".into()])
                .unwrap_err()
                .message()
                .contains("state_abbr")
        );
        assert!(test_call(&mut lib, &mut ctx, "nope", vec!["x".into()])
            .unwrap_err()
            .message()
            .contains("unknown function"));
        assert!(test_call(&mut lib, &mut ctx, "internet", vec![])
            .unwrap_err()
            .message()
            .contains("expected internet(kind)"));
        assert!(test_call(
            &mut lib,
            &mut ctx,
            "internet",
            vec!["email".into(), "extra".into()]
        )
        .unwrap_err()
        .message()
        .contains("expected internet(kind)"));
    }

    #[test]
    fn year_conversion_handles_gregorian_boundaries() {
        assert_eq!(year_from_unix_ms(0), 1970);
        assert_eq!(year_from_unix_ms(1_704_067_199_999), 2023);
        assert_eq!(year_from_unix_ms(1_704_067_200_000), 2024);
        assert_eq!(year_from_unix_ms(1_709_164_800_000), 2024);
        assert_eq!(year_from_unix_ms(1_735_689_600_000), 2025);
    }
}
