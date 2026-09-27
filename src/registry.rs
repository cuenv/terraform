use std::collections::{HashMap, HashSet};
use std::env;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::manifest::{self, Manifest, ProviderRelease};
use crate::registry_http;

#[derive(Deserialize)]
struct VersionsResponse {
    #[serde(default)]
    versions: Vec<RegistryVersion>,
}

#[derive(Deserialize)]
struct ProviderDetailsResponse {
    #[serde(default)]
    included: Vec<IncludedProviderVersion>,
}

#[derive(Deserialize)]
struct IncludedProviderVersion {
    #[serde(rename = "type")]
    resource_type: String,
    attributes: ProviderVersionAttributes,
}

#[derive(Deserialize)]
struct ProviderVersionAttributes {
    version: String,
    #[serde(rename = "published-at")]
    published_at: String,
}

#[derive(Deserialize)]
struct RegistryVersion {
    version: String,
    #[serde(default)]
    platforms: Vec<Platform>,
}

#[derive(Deserialize)]
struct Platform {
    os: String,
    arch: String,
}

pub fn newer_releases(
    client: &Client,
    manifest: &Manifest,
    provider_filter: Option<&str>,
) -> Result<Vec<ProviderRelease>> {
    let (target_os, target_arch) = terraform_platform()?;
    let release_cutoff = OffsetDateTime::now_utc() - time::Duration::days(30);
    let mut releases = Vec::new();

    for (provider_path, provider) in &manifest.providers {
        if provider_filter.is_some_and(|filter| filter != provider_path) {
            continue;
        }

        let (namespace, provider_type) = public_registry_address(&provider.source)?;
        let baseline = provider
            .versions
            .keys()
            .map(|version| Version::parse(version).context("parse configured provider version"))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max()
            .with_context(|| {
                format!("provider {provider_path:?} has no configured baseline version")
            })?;
        let url = format!(
            "https://registry.terraform.io/v1/providers/{namespace}/{provider_type}/versions"
        );
        let response: VersionsResponse = registry_http::get(client, &url, None)
            .with_context(|| format!("fetch release versions for {}", provider.source))?
            .error_for_status()
            .with_context(|| {
                format!(
                    "Terraform Registry returned an error for {}",
                    provider.source
                )
            })?
            .json()
            .with_context(|| {
                format!("parse Terraform Registry response for {}", provider.source)
            })?;
        let published_at = provider_release_dates(client, namespace, provider_type)?;

        let mut seen = HashSet::new();
        let mut versions = Vec::new();
        for registry_version in response.versions {
            let version = Version::parse(&registry_version.version).with_context(|| {
                format!(
                    "Terraform Registry returned invalid semantic version {:?} for {}",
                    registry_version.version, provider.source
                )
            })?;
            if !version.pre.is_empty() || !version.build.is_empty() || version <= baseline {
                continue;
            }
            let release_date = published_at
                .get(&registry_version.version)
                .with_context(|| {
                    format!(
                        "Terraform Registry has no publication date for {}@{}",
                        provider.source, registry_version.version
                    )
                })?;
            if *release_date < release_cutoff {
                continue;
            }
            if !registry_version
                .platforms
                .iter()
                .any(|platform| platform.os == target_os && platform.arch == target_arch)
            {
                bail!(
                    "Terraform Registry lists {}@{}, but it has no {target_os}_{target_arch} package for schema generation",
                    provider.source,
                    registry_version.version
                );
            }
            if seen.insert(registry_version.version.clone()) {
                versions.push((version, registry_version.version));
            }
        }
        versions.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, version) in versions {
            releases.push(manifest::release(manifest, provider_path, &version, None)?);
        }
    }

    Ok(releases)
}

fn provider_release_dates(
    client: &Client,
    namespace: &str,
    provider_type: &str,
) -> Result<HashMap<String, OffsetDateTime>> {
    let url = format!(
        "https://registry.terraform.io/v2/providers/{namespace}/{provider_type}?include=provider-versions"
    );
    let response: ProviderDetailsResponse = registry_http::get(client, &url, None)
        .with_context(|| format!("fetch provider release dates for {namespace}/{provider_type}"))?
        .error_for_status()
        .with_context(|| {
            format!("Terraform Registry returned an error for {namespace}/{provider_type}")
        })?
        .json()
        .with_context(|| {
            format!("parse Terraform Registry release dates for {namespace}/{provider_type}")
        })?;

    response
        .included
        .into_iter()
        .filter(|included| included.resource_type == "provider-versions")
        .map(|included| {
            let published_at = OffsetDateTime::parse(&included.attributes.published_at, &Rfc3339)
                .with_context(|| {
                format!(
                    "parse Terraform Registry publication date for {namespace}/{provider_type}@{}",
                    included.attributes.version
                )
            })?;
            Ok((included.attributes.version, published_at))
        })
        .collect()
}

fn terraform_platform() -> Result<(&'static str, &'static str)> {
    let os = match env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        other => bail!("unsupported Terraform provider platform OS {other:?}"),
    };
    let arch = match env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        "x86" => "386",
        other => bail!("unsupported Terraform provider platform architecture {other:?}"),
    };
    Ok((os, arch))
}

fn public_registry_address(source: &str) -> Result<(&str, &str)> {
    let parts: Vec<_> = source.split('/').collect();
    match parts.as_slice() {
        [namespace, provider_type] => Ok((namespace, provider_type)),
        ["registry.terraform.io", namespace, provider_type] => Ok((namespace, provider_type)),
        [host, _, _] => bail!(
            "automatic release discovery currently supports registry.terraform.io providers; {source:?} uses {host:?}"
        ),
        _ => bail!("invalid Terraform provider source {source:?}"),
    }
}
