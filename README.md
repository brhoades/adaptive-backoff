# Adaptive Backoff
Adaptive backoff provides a mechanism to intelligently back off use of rate
limited or failure operations through sources simples structs, `Backoffs`. Backoffs
take input on success and failure, then return a duration to wait.

For an adaptive backoff, as the failure and success calls increase the returned
duration eventually converges on a value to avoid rate limiting of requests.


## Example usage
Below is an example of an adapative `ExponentialBackoff` which works from a queue
and converges on a minimal delay duration between calls. It grows by a factor of `2.0` to
a max of `300`.

```rust
use adaptive_backoff::prelude::*;

let mut backoff = ExponentialBackoffBuilder::default()
    .factor(2.0)
    .max(300)
    .adaptive()?
    .build()
    .unwrap();

loop {
    let item = queue.pop();

    loop {
        match worker_iter(&conn, &item).await {
            Ok(_) => {
                backoff.success()?;
                delay_for(backoff.wait()?).await;
                break;
            }
            Err(_) => {
                backoff.fail()?;
                delay_for(backoff.wait()?).await;
            }
        }
    }
}
```
