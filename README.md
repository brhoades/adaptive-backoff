# Adaptive Backoff
Adaptive backoff provides a mechanism to intelligently back off use of rate
limited or failure operations through sources simples structs, `Backoffs`. Backoffs
take input on success and failure, then return a duration to wait.

For an adaptive backoff, as the failure and success calls increase the returned
duration eventually converges on a value to avoid rate limiting of requests.

## Usage
Include within your `Cargo.toml`:

```
adaptive_backoff = "0.2"
```

And follow an example below:

### Adaptive ExponentialBackoff
Below is an example of an adapative `ExponentialBackoff` which works from a queue
and converges on a minimal delay duration between calls. It grows by a factor of
`2.0` to a maximum of `300` seconds.

```rust
use std::time::Duration;
use adaptive_backoff::prelude::*;

let mut backoff = ExponentialBackoffBuilder::default()
    .factor(2.0)
    .max(Duration::from_secs(300))
    .adaptive()
    .build()
    .unwrap();

while let Some(item) = queue.pop() {
    loop {
        match worker_iter(&conn, &item).await {
            Ok(_) => {
                delay_for(backoff.success()).await;
                break;
            }
            Err(_) => delay_for(backoff.fail()).await,
        }
    }
}
```

### Simple ExponentialBackoff
If adaptive is omitted from the example above, a simple backoff is returned.
Its API lacks `success()` and `fail()`, instead it can only return increasing
delays with `wait()` until `reset()` is called to return it.

```rust
use std::time::Duration;
use adaptive_backoff::prelude::*;

let mut backoff = ExponentialBackoffBuilder::default()
    .factor(2.0)
    .max(Duration::from_secs(30))
    .build()
    .unwrap();

while let Some(item) = queue.pop() {
    loop {
        match worker_iter(&conn, &item).await {
            Ok(_) => {
                delay_for(backoff.wait()).await;
                break;
            }
            Err(_) => delay_for(backoff.wait()).await,
        }
    }

    backoff.reset();
}
```

### Additional Examples
There are tests for backoff implementations which contain example use
of the external API with expected output. See both the [simple exponential example](./src/lib/backoff.rs)
and the [adaptive exponential backoff example](./src/lib/adaptive.rs).
