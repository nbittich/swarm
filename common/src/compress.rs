use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use async_compression::tokio::bufread::GzipDecoder;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

pub async fn gzip(path: &Path, delete: bool) -> anyhow::Result<PathBuf> {
    if path.extension().is_some_and(|ext| ext == "gz") {
        return Ok(path.to_path_buf()); // no op, nothing to do
    }
    if !path.exists() || !path.is_file() {
        return Err(anyhow::anyhow!(
            "{path:?} doesn't exist or is not a file. Cannot gzip it"
        ));
    }
    use async_compression::tokio::write::GzipEncoder;
    let extension = path.extension().and_then(|ex| ex.to_str()).unwrap_or("");

    let gzip_path = path.with_extension(format!("{extension}.gz"));
    let input_file = tokio::fs::File::open(path).await?;
    let output_file = tokio::fs::File::create(&gzip_path).await?;
    let mut encoder = GzipEncoder::new(output_file);
    let mut buf = BufReader::new(input_file);
    tokio::io::copy_buf(&mut buf, &mut encoder).await?;

    encoder.shutdown().await?;
    if delete {
        tokio::fs::remove_file(path).await?;
    }
    Ok(gzip_path)
}
pub async fn ungzip(path: &Path, buffer: &mut String) -> anyhow::Result<()> {
    super::debug!("ungzip: reading {path:?}");
    buffer.clear();
    if path.extension().is_some_and(|ext| ext == "gz") {
        let mut gz_buff = Vec::with_capacity(1024);
        let mut f = tokio::fs::File::open(path).await?;
        f.read_to_end(&mut gz_buff).await?;
        let cursor = Cursor::new(gz_buff);
        let mut reader = BufReader::new(cursor);
        let mut decoder = GzipDecoder::new(&mut reader);
        decoder.read_to_string(buffer).await?;
    } else {
        tokio::fs::File::open(path)
            .await?
            .read_to_string(buffer)
            .await?;
    }
    Ok(())
}
