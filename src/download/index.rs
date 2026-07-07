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

#[context("finding a file matching version {version} in the index at `{index}`")]
pub(crate) fn find_in_index(
    index: &Url,
    version: &VersionRequirement,
    file_name_prefix: &str,
) -> anyhow::Result<(Url, Compression)> {
    let response = reqwest::blocking::get(index.clone())?;

    let response = response.error_for_status()?;

    // We may get redirected.
    let resolved_index = response.url().clone();

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
        bail!("found file matching version {version}");
    };

    let url = resolved_index
        .join(file_name)
        .context("joining the file name to the index url")?;

    info!("resolved index `{index}` with version {version} to `{url}`");

    Ok((url, compression))
}
