use crate::Compression;
use crate::HostPath;
use anyhow::Context as _;
use fn_error_context::context;
use lzma_rs::xz_decompress;
use reqwest::blocking::ClientBuilder;
use std::io::Cursor;
use tar::Archive;
use url::Url;

#[context("downloading the tarball from `{url}`")]
pub(in crate::download) fn download_tarball(
    url: &Url,
    compression: Compression,
    source_directory: &HostPath,
) -> anyhow::Result<()> {
    let client = ClientBuilder::new()
        .timeout(None)
        .build()
        .context("initialising the http client")?;

    let response = client.get(url.clone()).send()?;

    let response = response.error_for_status()?;

    let compressed_bytes = response.bytes()?;

    let mut decompressed_bytes = Vec::new();

    match compression {
        Compression::None => decompressed_bytes = compressed_bytes.to_vec(),
        Compression::Xz => {
            xz_decompress(&mut &*compressed_bytes, &mut decompressed_bytes)
                .context("decompressing the tarball")?;
        }
    }

    let mut archive = Archive::new(Cursor::new(decompressed_bytes));

    archive.unpack(source_directory)?;

    Ok(())
}

pub(crate) fn split_tarball_file_name(file_name: &str) -> Option<(&str, Compression)> {
    let (basename, extension) = file_name.rsplit_once(".tar")?;

    if extension.contains('/') {
        return None;
    }

    let extension = extension.strip_prefix(".");

    let compression = Compression::from_extension(extension)?;

    Some((basename, compression))
}

pub(crate) fn detect_tarball_compression(url: &str) -> Option<Compression> {
    let (_basename, compression) = split_tarball_file_name(url)?;
    Some(compression)
}
