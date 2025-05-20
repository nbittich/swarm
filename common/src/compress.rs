use std::{
    fmt::Debug,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_compression::tokio::bufread::GzipDecoder;
use swarm_retryable_fut::retryable_fut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::instrument;

use crate::retry_fs;

#[instrument(level = "debug")]
pub async fn gzip_str<'a>(s: &'a str) -> anyhow::Result<Vec<u8>> {
    use async_compression::tokio::write::GzipEncoder;

    if s.is_empty() {
        return Ok(Vec::new());
    }

    let mut gzipped_s = Vec::with_capacity(std::cmp::max(32, s.len() / 8));
    let mut encoder = GzipEncoder::new(&mut gzipped_s);

    encoder.write_all(s.as_bytes()).await?;
    encoder.shutdown().await?;

    Ok(gzipped_s)
}

#[instrument(level = "debug")]
pub async fn gzip(path: &Path, delete: bool) -> anyhow::Result<PathBuf> {
    if path.extension().is_some_and(|ext| ext == "gz") {
        return Ok(path.to_path_buf()); // no op, nothing to do
    }
    if !path.exists() || !path.is_file() {
        return Err(anyhow::anyhow!(
            "{path:?} doesn't exist or is not a file. Cannot gzip it"
        ));
    }
    let path = Arc::new(path.to_path_buf());

    let inner_path = path.clone();
    retryable_fut(
        *retry_fs::MAX_RETRY,
        *retry_fs::DELAY_BEFORE_NEXT_RETRY,
        async move || {
            let extension = inner_path
                .extension()
                .and_then(|ex| ex.to_str())
                .unwrap_or("");
            let gzip_path = inner_path.with_extension(format!("{extension}.gz"));
            use async_compression::tokio::write::GzipEncoder;
            let input_file = tokio::fs::File::open(inner_path.as_ref()).await?;
            let output_file = tokio::fs::File::create(&gzip_path).await?;
            let mut encoder = GzipEncoder::new(output_file);
            let mut buf = BufReader::new(input_file);
            tokio::io::copy_buf(&mut buf, &mut encoder).await?;
            encoder.shutdown().await?;
            if delete {
                tokio::fs::remove_file(path.as_ref()).await?;
            }
            Ok(gzip_path)
        },
    )
    .await
}
#[instrument(level = "debug")]
pub async fn ungzip(path: impl AsRef<Path> + Debug) -> anyhow::Result<String> {
    let path = path.as_ref().to_path_buf();
    retryable_fut(
        *retry_fs::MAX_RETRY,
        *retry_fs::DELAY_BEFORE_NEXT_RETRY,
        async move || {
            let path = &path;
            super::debug!("ungzip: reading {path:?}");
            let mut buffer = String::with_capacity(1024);
            if path.extension().is_some_and(|ext| ext == "gz") {
                let mut gz_buff = Vec::with_capacity(1024);
                let mut f = retry_fs::open_file(path).await?;
                f.read_to_end(&mut gz_buff).await?;
                let cursor = Cursor::new(gz_buff);
                let mut reader = BufReader::new(cursor);
                let mut decoder = GzipDecoder::new(&mut reader);
                decoder.read_to_string(&mut buffer).await?;
            } else {
                tokio::fs::File::open(path)
                    .await?
                    .read_to_string(&mut buffer)
                    .await?;
            }
            Ok(buffer)
        },
    )
    .await
}
