use std::{path::PathBuf, time::Duration};

use anyhow::anyhow;
use reqwest::{
    Client,
    header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT},
};
use swarm_common::constant::{
    BUFFER_BACK_PRESSURE, CONNECTION_POOL_MAX_IDLE_PER_HOST, DEFAULT_ACCEPT, DEFAULT_USER_AGENT,
    INTERESTING_PROPERTIES, MAX_DELAY_BEFORE_NEXT_RETRY_MILLIS, MAX_DELAY_MILLIS, MAX_RETRY,
    MIN_DELAY_BEFORE_NEXT_RETRY_MILLIS, MIN_DELAY_MILLIS, REQUEST_TIMEOUT_SEC,
};

// static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
//     std::env::var(HTTP_CACHE_PATH)
//         .map(PathBuf::from)
//         .unwrap_or_else(|_| temp_dir())
// });

#[derive(Debug, Clone)]
pub struct Configuration {
    pub max_retry: usize,
    pub client: Client,
    pub folder_path: PathBuf,
    pub ignore_extensions: [&'static str; 8],
    pub allowed_content_types: [&'static str; 1],
    pub href_selector: scraper::Selector,
    pub redirect_selector: scraper::Selector,
    // pub job_timeout: Duration,
    pub buffer: usize,
    pub min_delay_millis: u64,
    pub max_delay_millis: u64,
    pub min_delay_before_next_retry_millis: u64,
    pub max_delay_before_next_retry_millis: u64,
    pub interesting_properties: Option<Vec<String>>,
}
pub fn get_reqwest_client() -> anyhow::Result<Client> {
    let default_user_agent = std::env::var(DEFAULT_USER_AGENT).unwrap_or_else(|_| {
        "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0".into()
    });
    let default_accept = std::env::var(DEFAULT_ACCEPT).unwrap_or_else(|_| {
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".into()
    });
    let timeout = std::env::var(REQUEST_TIMEOUT_SEC)
        .unwrap_or_else(|_| "30".into())
        .parse::<u64>()?;
    let pool_max_idle_per_host = std::env::var(CONNECTION_POOL_MAX_IDLE_PER_HOST)
        .ok()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .pool_max_idle_per_host(pool_max_idle_per_host)
        .default_headers(HeaderMap::from_iter(
            [
                (USER_AGENT, HeaderValue::from_str(&default_user_agent)?),
                (ACCEPT, HeaderValue::from_str(&default_accept)?),
            ]
            .into_iter(),
        ))
        .build()?;
    Ok(client)
}
pub async fn make_config(client: Client, folder_path: PathBuf) -> anyhow::Result<Configuration> {
    let max_retry = std::env::var(MAX_RETRY)
        .unwrap_or_else(|_| "3".into())
        .parse::<usize>()?;

    let mut min_delay_before_next_retry_millis = std::env::var(MIN_DELAY_BEFORE_NEXT_RETRY_MILLIS)
        .unwrap_or_else(|_| "200".into())
        .parse::<u64>()?;
    let mut max_delay_before_next_retry_millis = std::env::var(MAX_DELAY_BEFORE_NEXT_RETRY_MILLIS)
        .unwrap_or_else(|_| "500".into())
        .parse::<u64>()?;
    if min_delay_before_next_retry_millis == 0 {
        min_delay_before_next_retry_millis = 30; // cannot be 0
    }
    if max_delay_before_next_retry_millis == 0 {
        max_delay_before_next_retry_millis = 30; // cannot be 0
    }
    let mut min_delay_millis = std::env::var(MIN_DELAY_MILLIS)
        .unwrap_or_else(|_| "20".into())
        .parse::<u64>()?;
    let mut max_delay_millis = std::env::var(MAX_DELAY_MILLIS)
        .unwrap_or_else(|_| "250".into())
        .parse::<u64>()?;
    if max_delay_millis == 0 {
        max_delay_millis = 30; // cannot be 0
    }
    if min_delay_millis == 0 {
        min_delay_millis = 30; // cannot be 0
    }

    let redirect_selector =
        scraper::Selector::parse(r#"meta[http-equiv="refresh"]"#).map_err(|e| anyhow!("{e}"))?;

    let href_selector = scraper::Selector::parse("a").map_err(|e| anyhow!("{e}"))?;
    if folder_path.exists() {
        tokio::fs::remove_dir_all(&folder_path).await?;
    }
    tokio::fs::create_dir_all(&folder_path).await?;
    let ignore_extensions = [
        ".js", ".css", ".pdf", ".jpg", ".png", ".docx", ".csv", ".xlsx",
    ];
    let allowed_content_types = ["text/html"];
    //let job_timeout = Duration::from_secs((max_retry as u64 * timeout) + (max_delay_sec * 3));
    let buffer = std::env::var(BUFFER_BACK_PRESSURE)
        .unwrap_or_else(|_| "16".into())
        .parse::<usize>()?;
    let interesting_properties = std::env::var(INTERESTING_PROPERTIES)
        .map(|ip| ip.split(",").map(|p| p.trim().to_lowercase()).collect())
        .ok();
    Ok(Configuration {
        max_retry,
        href_selector,
        //job_timeout,
        buffer,
        allowed_content_types,
        min_delay_millis,
        max_delay_millis,
        redirect_selector,
        min_delay_before_next_retry_millis,
        max_delay_before_next_retry_millis,
        interesting_properties,
        ignore_extensions,
        folder_path,
        client,
    })
}
