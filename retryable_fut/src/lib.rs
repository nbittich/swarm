use std::{fmt::Display, time::Duration};

use tracing::{debug, error};

#[allow(unreachable_code)]
pub async fn retryable_fut<O, E>(
    max_retry: u64,
    delay_before_next_retry: u64,
    f: impl AsyncFn() -> Result<O, E>,
) -> Result<O, E>
where
    E: Display,
{
    let mut count = 0;

    loop {
        match f().await {
            v @ Ok(_) => return v,
            Err(e) => {
                count += 1;
                error!("could not execute function. Error: {e}. retry count: {count}");

                if count == max_retry {
                    return Err(e);
                }
                let sleep_time = count * delay_before_next_retry;

                debug!("sleeping {sleep_time} seconds before next retry...");
                tokio::time::sleep(Duration::from_secs(sleep_time)).await;
            }
        }
    }
    unreachable!()
}
