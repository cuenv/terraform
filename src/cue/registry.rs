use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::LINK;
use serde::Deserialize;

use crate::registry::http;

const REGISTRY_ORIGIN: &str = "https://registry.cue.works";

#[derive(Deserialize)]
struct TagsResponse {
    #[serde(default)]
    tags: Option<Vec<String>>,
}

pub fn published_tags(
    client: &Client,
    module_path: &str,
    bearer_token: &str,
) -> Result<HashSet<String>> {
    let repository = repository_name(module_path)?;
    let mut next = Some(format!("{REGISTRY_ORIGIN}/v2/{repository}/tags/list?n=100"));
    let mut visited = HashSet::new();
    let mut tags = HashSet::new();
    let mut first_page = true;

    while let Some(url) = next.take() {
        if !visited.insert(url.clone()) {
            bail!("CUE Registry repeated tag-list page {url}");
        }
        let response = http::get(client, &url, Some(bearer_token))
            .with_context(|| format!("list CUE Registry tags for {repository}"))?;
        if response.status() == StatusCode::NOT_FOUND && first_page {
            return Ok(tags);
        }
        let response = response.error_for_status().with_context(|| {
            format!("CUE Registry returned an error while listing tags for {repository}")
        })?;
        next = next_page(&response)?;
        let page: TagsResponse = response
            .json()
            .with_context(|| format!("parse CUE Registry tags for {repository}"))?;
        tags.extend(page.tags.unwrap_or_default());
        first_page = false;
    }

    Ok(tags)
}

pub(crate) fn repository_name(module_path: &str) -> Result<&str> {
    let (repository, major) = module_path
        .rsplit_once("@v")
        .with_context(|| format!("CUE module path {module_path:?} has no @v major suffix"))?;
    if repository.is_empty() || major.is_empty() || !major.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("invalid CUE module path {module_path:?}");
    }
    Ok(repository)
}

fn next_page(response: &Response) -> Result<Option<String>> {
    let Some(link_header) = response.headers().get(LINK) else {
        return Ok(None);
    };
    let link_header = link_header
        .to_str()
        .context("CUE Registry returned an invalid Link header")?;

    for link in link_header.split(',') {
        let Some((target, parameters)) = link.split_once(';') else {
            continue;
        };
        if !parameters.contains("rel=\"next\"") && !parameters.contains("rel=next") {
            continue;
        }
        let target = target
            .trim()
            .strip_prefix('<')
            .and_then(|value| value.strip_suffix('>'))
            .context("CUE Registry returned a malformed next-page Link")?;
        let next = response
            .url()
            .join(target)
            .context("resolve CUE Registry next-page Link")?;
        if next.scheme() != "https"
            || next.host_str() != Some("registry.cue.works")
            || !next.path().starts_with("/v2/")
        {
            bail!("CUE Registry returned an unsafe next-page Link: {next}");
        }
        return Ok(Some(next.to_string()));
    }

    Ok(None)
}

pub fn release_tag(provider_version: &str) -> String {
    format!("v{provider_version}")
}

#[cfg(test)]
mod tests {
    use super::{release_tag, repository_name};

    #[test]
    fn strips_module_major_to_get_registry_repository() {
        assert_eq!(
            repository_name("github.com/cuenv/terraform/terraform/cloudflare/cloudflare@v5")
                .expect("valid CUE module path"),
            "github.com/cuenv/terraform/terraform/cloudflare/cloudflare"
        );
    }

    #[test]
    fn uses_the_terraform_release_as_the_module_tag() {
        assert_eq!(release_tag("6.66.0"), "v6.66.0");
    }
}
