use crate::Compression;
use crate::Resolver;
use crate::Version;
use crate::VersionRequirement;
use crate::download::split_tarball_file_name;
use anyhow::Context as _;
use anyhow::bail;
use fn_error_context::context;
use std::str::from_utf8;
use tl::Node;
use tl::ParserOptions;
use tracing::info;
use url::Url;

pub(crate) struct IndexedFile {
    pub real_url: Url,
    pub virtual_url: Option<Url>,
    pub compression: Compression,
}

#[context("finding a file matching version {version} in the index at `{index}`")]
pub(crate) fn find_in_index(
    index: &Url,
    version: &VersionRequirement,
    file_name_prefix: &str,
) -> anyhow::Result<IndexedFile> {
    let response = reqwest::blocking::get(index.clone())?;

    let response = response.error_for_status()?;

    // We may get redirected.
    let resolved_index = response.url().clone();
    let virtual_index = (resolved_index != *index).then_some(index);

    let bytes = response.bytes()?;
    let string = from_utf8(&bytes).context("parsing the HTML as UTF-8")?;

    let dom = tl::parse(string, ParserOptions::new()).context("parsing the HTML")?;

    // TODO: Set favouring of compression types.
    let mut resolver = Resolver::from_requirement(version);

    // TODO: Sprinkle in some logs.
    for node in dom.nodes() {
        let Node::Tag(tag) = node else {
            continue;
        };

        if tag.name() != "a" {
            continue;
        }

        let Some(Some(file_name)) = tag.attributes().get("href") else {
            continue;
        };

        let Ok(file_name) = from_utf8(file_name.as_bytes()) else {
            // This should be unreachable since we pass in utf-8...
            continue;
        };

        let Some((basename, compression)) = split_tarball_file_name(file_name) else {
            continue;
        };

        let Some(version) = basename.strip_prefix(file_name_prefix) else {
            continue;
        };

        let version = Version::from(version);

        resolver.add_option((file_name, compression), version);
    }

    let Some((file_name, compression)) = resolver.best() else {
        bail!("no file matching version {version} was found");
    };

    let real_url = resolved_index
        .join(file_name)
        .context("joining the file name to the index url")?;

    let virtual_url = virtual_index
        .map(|index| index.join(file_name))
        .transpose()
        .context("joining the file name to the index url")?;

    info!("resolved index `{index}` with version {version} to `{real_url}`");

    Ok(IndexedFile {
        real_url,
        virtual_url,
        compression,
    })
}
