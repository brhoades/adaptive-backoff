use error_chain::*;

error_chain! {
    errors {
        AdaptiveBuilder(t: String) {
            description("cannot build adaptive backoff")
            display("unable to build adaptive backoff: {}", t)
        }
        AdaptiveBaseDelay {
            description("missing base delay for adaptive backoff")
            display("missing base delay for adaptive backoff")
        }
    }
}
