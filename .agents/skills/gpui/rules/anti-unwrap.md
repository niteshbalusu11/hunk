---
title: Avoid unwrap(), Use ? or Explicit Handling
impact: HIGH
tags: error, panic, unwrap
---

## Panic Prevention

`unwrap()` panics on None/Err, crashing your application.

**Incorrect (panics in production):**

```rust
// BAD: Panics on None
let value = option.unwrap();
let item = items[index];  // Panics on out of bounds

// BAD: Panics on Err
let result = fallible_fn().unwrap();
```

**Correct (handle gracefully):**

```rust
// GOOD: Propagate with ?
let value = option.ok_or_else(|| anyhow!("missing value"))?;
let result = fallible_fn()?;

// GOOD: Provide default
let value = option.unwrap_or_default();
let value = option.unwrap_or(fallback);

// GOOD: Handle error explicitly
let result = fallible_fn().unwrap_or_else(|e| {
    log::error!("Failed: {}", e);
    default_value
});

// GOOD: Safe array access
let item = items.get(index);  // Returns Option<&T>
let item = items.get(index).unwrap_or(&default);
```

### Context for Errors

```rust
// Add context when propagating
let data = fetch_data()
    .await
    .context("Failed to fetch data")?;

let config = load_config()
    .context("Failed to load configuration")?;
```

### When unwrap() is Acceptable

```rust
// OK: In tests
#[test]
fn test_something() {
    let result = function().unwrap();
    assert_eq!(result, expected);
}

// OK: Compile-time known values
let regex = Regex::new(r"^\d+$").unwrap();  // Constant pattern

// OK: After explicit check
if option.is_some() {
    let value = option.unwrap();  // Safe, but prefer if-let
}

// Better: Use if-let or match
if let Some(value) = option {
    // use value
}
```

### ResultExt Helpers

```rust
// Log and continue with Option
if let Some(data) = fetch().await.log_err() {
    process(data);
}

// Assert in debug, log in release
result.debug_assert_ok("Should never fail");
```
