use std::collections::HashSet;
use std::env;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, LINK};
use semver::Version;
use serde::Deserialize;

use crate::registry_http;

const DEFAULT_REGISTRY_URL: &str = "https://registry.cue.works";
const TAGS_PER_PAGE: usize = 100;

#[derive(Deserialize)]
struct TagsResponse {
    name: String,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

pub fn is_published(client: &Client, module_path: &str) -> Result<bool> {
    let (repository, module_major) = module_repository(module_path)?;
    let registry_url = registry_url()?;
    let mut next_page = Some(format!(
        "{registry_url}/v2/{repository}/tags/list?n={TAGS_PER_PAGE}"
    ));
    let mut visited_pages = HashSet::new();
    let token = env::var("CUE_REGISTRY_TOKEN").ok();

    while let Some(url) = next_page.take() {
        if !visited_pages.insert(url.clone()) {
            bail!("CUE Registry repeated a tags-list pagination link for {repository}");
        }

        let response = get_tags_page(client, &url, &registry_url, token.as_deref())?;
        if response.status != 200 {
            let body = String::from_utf8_lossy(&response.body);
            if matches!(response.status, 401 | 403) {
                bail!(
                    "CUE Registry denied access to {repository}; confirm the module is publicly readable or set CUE_REGISTRY_TOKEN"
                );
            }
            bail!(
                "CUE Registry returned HTTP {} while listing tags for {repository}: {}",
                response.status,
                body.trim()
            );
        }

        let page: TagsResponse = serde_json::from_slice(&response.body)
            .with_context(|| format!("parse CUE Registry tags for {repository}"))?;
        if page.name != repository {
            bail!(
                "CUE Registry returned tags for {:?} while querying {repository}",
                page.name
            );
        }
        if let Some(tags) = page.tags {
            for tag in tags {
                let version = tag.strip_prefix('v').with_context(|| {
                    format!("CUE Registry returned a non-CUE module tag {tag:?}")
                })?;
                let version = Version::parse(version).with_context(|| {
                    format!("CUE Registry returned an invalid module tag {tag:?}")
                })?;
                if version.major != module_major {
                    bail!(
                        "CUE Registry returned tag {tag:?} for module {module_path}, whose major version is v{module_major}"
                    );
                }
                return Ok(true);
            }
        }

        next_page = next_link(&response.headers, &registry_url)?;
    }

    Ok(false)
}

struct TagsPage {
    status: u16,
    headers: HeaderMap,
    body: Vec<u8>,
}

fn get_tags_page(
    client: &Client,
    url: &str,
    registry_url: &str,
    token: Option<&str>,
) -> Result<TagsPage> {
    let response = registry_http::get(client, url, token)
        .with_context(|| format!("query CUE Registry {registry_url}"))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response
        .bytes()
        .context("read CUE Registry response body")?
        .to_vec();
    Ok(TagsPage {
        status,
        headers,
        body,
    })
}

fn module_repository(module_path: &str) -> Result<(&str, u64)> {
    let (repository, major) = module_path
        .rsplit_once('@')
        .with_context(|| format!("CUE module path {module_path:?} is missing its major version"))?;
    let major = major
        .strip_prefix('v')
        .with_context(|| format!("invalid CUE module major version in {module_path:?}"))?
        .parse::<u64>()
        .with_context(|| format!("invalid CUE module major version in {module_path:?}"))?;
    if repository.is_empty() || repository.contains('@') {
        bail!("invalid CUE module path {module_path:?}");
    }
    Ok((repository, major))
}

fn registry_url() -> Result<String> {
    let value = env::var("CUE_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_owned());
    let value = value.trim_end_matches('/');
    let local_http = value == "http://localhost"
        || value.starts_with("http://localhost:")
        || value.starts_with("http://localhost/");
    if !(value.starts_with("https://") || local_http)
        || value
            .chars()
            .any(|character| matches!(character, '?' | '#' | ' ' | '\n' | '\r'))
    {
        bail!("CUE_REGISTRY_URL must be an HTTPS registry URL (HTTP is allowed for localhost)");
    }
    Ok(value.to_owned())
}

fn next_link(headers: &HeaderMap, registry_url: &str) -> Result<Option<String>> {
    for value in headers.get_all(LINK) {
        let value = value
            .to_str()
            .context("parse CUE Registry pagination header")?;
        for link in value.split(',') {
            if !link.contains("rel=\"next\"") && !link.contains("rel=next") {
                continue;
            }
            let start = link
                .find('<')
                .context("parse CUE Registry pagination link")?;
            let end = link[start + 1..]
                .find('>')
                .map(|offset| start + 1 + offset)
                .context("parse CUE Registry pagination link")?;
            let target = &link[start + 1..end];
            if target.starts_with('/') {
                return Ok(Some(format!("{registry_url}{target}")));
            }
            if target.starts_with(&format!("{registry_url}/")) {
                return Ok(Some(target.to_owned()));
            }
            bail!("CUE Registry returned a pagination link outside {registry_url}");
        }
    }
    Ok(None)
}
