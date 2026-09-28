use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, LINK};
use serde::Deserialize;

use crate::registry::http;
use crate::util::sha256;

const REGISTRY_ORIGIN: &str = "https://ghcr.io";
const MODULE_PREFIX: &str = "ghcr.io/cuenv/";
const GHCR_REPOSITORY: &str = "cuenv/terraform-cue";

#[derive(Deserialize)]
struct TagsResponse {
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RegistryTokenResponse {
    token: Option<String>,
    access_token: Option<String>,
}

pub struct GhcrRegistry<'a> {
    client: &'a Client,
    repository: String,
    bearer_token: String,
}

impl<'a> GhcrRegistry<'a> {
    pub fn connect(
        client: &'a Client,
        module_path: &str,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let repository = repository_name(module_path)?;
        let bearer_token = registry_token(client, &repository, username, password)?;
        Ok(Self {
            client,
            repository,
            bearer_token,
        })
    }

    pub fn tags(&self) -> Result<HashSet<String>> {
        let mut next = Some(format!(
            "{REGISTRY_ORIGIN}/v2/{}/tags/list?n=100",
            self.repository
        ));
        let mut visited = HashSet::new();
        let mut tags = HashSet::new();
        let mut first_page = true;

        while let Some(url) = next.take() {
            if !visited.insert(url.clone()) {
                bail!("GHCR repeated tag-list page {url}");
            }
            let response = http::get(self.client, &url, Some(&self.bearer_token))
                .with_context(|| format!("list GHCR tags for {}", self.repository))?;
            if response.status() == StatusCode::NOT_FOUND && first_page {
                return Ok(tags);
            }
            let response = response.error_for_status().with_context(|| {
                format!(
                    "GHCR returned an error while listing tags for {}",
                    self.repository
                )
            })?;
            next = next_page(&response)?;
            let page: TagsResponse = response
                .json()
                .with_context(|| format!("parse GHCR tags for {}", self.repository))?;
            tags.extend(page.tags.unwrap_or_default());
            first_page = false;
        }

        Ok(tags)
    }

    pub fn tag_exists(&self, tag: &str) -> Result<bool> {
        let url = format!("{REGISTRY_ORIGIN}/v2/{}/manifests/{tag}", self.repository);
        let response = self
            .client
            .head(url)
            .bearer_auth(&self.bearer_token)
            .header(
                ACCEPT,
                "application/vnd.oci.image.manifest.v1+json, application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json",
            )
            .send()
            .with_context(|| format!("check GHCR tag {tag}"))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        response
            .error_for_status()
            .with_context(|| format!("GHCR returned an error checking tag {tag}"))?;
        Ok(true)
    }
}

pub fn published_tags(
    client: &Client,
    module_path: &str,
    username: &str,
    password: &str,
) -> Result<HashSet<String>> {
    GhcrRegistry::connect(client, module_path, username, password)?.tags()
}

pub(crate) fn repository_name(module_path: &str) -> Result<String> {
    let (module_path, major) = module_path
        .rsplit_once("@v")
        .with_context(|| format!("CUE module path {module_path:?} has no @v major suffix"))?;
    if !module_path.starts_with(MODULE_PREFIX)
        || major.is_empty()
        || !major.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("invalid CUE module path {module_path:?}");
    }
    Ok(GHCR_REPOSITORY.to_owned())
}

pub(crate) fn registry_tag(module_path: &str, module_version: &str) -> Result<String> {
    let repository_path = module_path
        .rsplit_once("@v")
        .map(|(path, _)| path)
        .with_context(|| format!("CUE module path {module_path:?} has no @v major suffix"))?;
    repository_name(module_path)?;
    Ok(format!(
        "{}-{module_version}",
        sha256(repository_path.as_bytes())
    ))
}

fn registry_token(
    client: &Client,
    repository: &str,
    username: &str,
    password: &str,
) -> Result<String> {
    let scope = format!("repository:{repository}:pull");
    let mut request = client
        .get(format!("{REGISTRY_ORIGIN}/token"))
        .query(&[("service", "ghcr.io"), ("scope", scope.as_str())]);
    if !username.is_empty() || !password.is_empty() {
        request = request.basic_auth(username, Some(password));
    }
    let response: RegistryTokenResponse = request
        .send()
        .context("request GHCR registry token")?
        .error_for_status()
        .context("GHCR rejected registry authentication")?
        .json()
        .context("parse GHCR registry token")?;
    response
        .token
        .or(response.access_token)
        .context("GHCR returned no registry token")
}

fn next_page(response: &Response) -> Result<Option<String>> {
    let Some(link_header) = response.headers().get(LINK) else {
        return Ok(None);
    };
    let link_header = link_header
        .to_str()
        .context("GHCR returned an invalid Link header")?;

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
            .context("GHCR returned a malformed next-page Link")?;
        let next = response
            .url()
            .join(target)
            .context("resolve GHCR next-page Link")?;
        if next.scheme() != "https"
            || next.host_str() != Some("ghcr.io")
            || !next.path().starts_with("/v2/")
        {
            bail!("GHCR returned an unsafe next-page Link: {next}");
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
            repository_name("ghcr.io/cuenv/terraform/cloudflare/cloudflare@v5")
                .expect("valid CUE module path"),
            "cuenv/terraform-cue"
        );
    }

    #[test]
    fn uses_the_terraform_release_as_the_module_tag() {
        assert_eq!(release_tag("6.66.0"), "v6.66.0");
    }
}
