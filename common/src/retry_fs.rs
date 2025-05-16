use std::{
    fmt::Debug,
    path::Path,
    sync::{Arc, LazyLock},
};

use tokio::{
    fs::{OpenOptions, ReadDir},
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::instrument;

use super::retryable_fut;

use crate::constant::{FS_DELAY_BEFORE_NEXT_RETRY, FS_MAX_RETRY};

#[instrument(level = "debug")]
pub async fn read_to_end(path: impl AsRef<Path> + Debug) -> Result<Vec<u8>, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        let mut file = tokio::fs::File::open(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        Ok(data)
    })
    .await
}

#[instrument(level = "debug")]
pub async fn read_to_string(path: impl AsRef<Path> + Debug) -> Result<String, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::read_to_string(&path).await
    })
    .await
}

#[instrument(level = "debug")]
pub async fn read_dir(path: impl AsRef<Path> + Debug) -> Result<ReadDir, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::read_dir(&path).await
    })
    .await
}

#[instrument(level = "debug")]
pub async fn append_to_file(
    path: impl AsRef<Path> + Debug,
    line: String,
) -> Result<(), std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        let mut manifest_file = tokio::fs::File::options()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        manifest_file.write_all(line.as_bytes()).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn remove_dir_all(path: impl AsRef<Path> + Debug) -> Result<(), std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::remove_dir_all(&path).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn create_dir_all(path: impl AsRef<Path> + Debug) -> Result<(), std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::create_dir_all(&path).await
    })
    .await
}

#[instrument(level = "debug")]
pub async fn open_file(path: impl AsRef<Path> + Debug) -> Result<tokio::fs::File, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::File::open(&path).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn create_file(
    path: impl AsRef<Path> + Debug,
) -> Result<tokio::fs::File, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::File::create(&path).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn remove_file(path: impl AsRef<Path> + Debug) -> Result<(), std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::remove_file(&path).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn open_file_with_options(
    path: impl AsRef<Path> + Debug,
    options: OpenOptions,
) -> Result<tokio::fs::File, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        options.open(&path).await
    })
    .await
}
#[instrument(level = "debug")]
pub async fn write(path: impl AsRef<Path> + Debug, b: Arc<String>) -> Result<(), std::io::Error> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::write(&path, b.as_bytes()).await
    })
    .await
}
pub async fn copy(
    from: impl AsRef<Path> + Debug,
    to: impl AsRef<Path> + Debug,
) -> Result<u64, std::io::Error> {
    let from = from.as_ref().to_path_buf();
    let to = to.as_ref().to_path_buf();
    retryable_fut(*MAX_RETRY, *DELAY_BEFORE_NEXT_RETRY, async move || {
        tokio::fs::copy(&from, &to).await
    })
    .await
}
pub static MAX_RETRY: LazyLock<u64> = LazyLock::new(|| {
    std::env::var(FS_MAX_RETRY)
        .unwrap_or_else(|_| "5".into())
        .parse::<u64>()
        .unwrap_or(5)
});

pub static DELAY_BEFORE_NEXT_RETRY: LazyLock<u64> = LazyLock::new(|| {
    std::env::var(FS_DELAY_BEFORE_NEXT_RETRY)
        .unwrap_or_else(|_| "10".into())
        .parse::<u64>()
        .unwrap_or(15000)
});
