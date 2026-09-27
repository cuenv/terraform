use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use cuengine::evaluate_cue_package_typed;
use semver::Version;
use serde::Deserialize;

const CUEENGINE_LANGUAGE_VERSION: &str = "v0.16.0";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub module_prefix: String,
    pub module_major: u64,
    pub cue_language_version: String,
    pub terraform_cli_version: String,
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Debug, Deserialize)]
pub struct Provider {
    pub source: String,
    pub versions: BTreeMap<String, Release>,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    #[serde(default)]
    pub path_version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProviderRelease {
    pub provider_path: String,
    pub source: String,
    pub provider_version: String,
    pub module_path: String,
    pub path_version: String,
}

pub fn load(root: &Path) -> Result<Manifest> {
    let manifest: Manifest = evaluate_cue_package_typed(root, "providers")
        .context("evaluate providers.cue with cuengine")?;
    validate(&manifest)?;
    Ok(manifest)
}

pub fn releases(manifest: &Manifest) -> Result<Vec<ProviderRelease>> {
    let mut result = Vec::new();
    let mut module_paths = HashSet::new();

    for (provider_path, provider) in &manifest.providers {
        for (provider_version, configured_release) in &provider.versions {
            let generated = release(
                manifest,
                provider_path,
                provider_version,
                configured_release.path_version.as_deref(),
            )?;
            let module_path = generated.module_path.clone();
            if !module_paths.insert(module_path.clone()) {
                bail!("duplicate CUE module path {module_path:?}");
            }
            result.push(generated);
        }
    }
    Ok(result)
}

pub fn release(
    manifest: &Manifest,
    provider_path: &str,
    provider_version: &str,
    path_version: Option<&str>,
) -> Result<ProviderRelease> {
    let provider = manifest
        .providers
        .get(provider_path)
        .with_context(|| format!("unknown provider path {provider_path:?}"))?;
    let path_version = path_version.unwrap_or(provider_version);
    validate_module_segment(path_version).with_context(|| {
        format!("{provider_path}@{provider_version} has invalid path version {path_version:?}")
    })?;
    let module_path = format!(
        "{}/{}/{}@v{}",
        manifest.module_prefix, provider_path, path_version, manifest.module_major
    );
    if module_path.len() > 128 {
        bail!("CUE module path is longer than 128 characters: {module_path}");
    }
    Ok(ProviderRelease {
        provider_path: provider_path.to_owned(),
        source: provider.source.clone(),
        provider_version: provider_version.to_owned(),
        module_path,
        path_version: path_version.to_owned(),
    })
}

fn validate(manifest: &Manifest) -> Result<()> {
    validate_module_prefix(&manifest.module_prefix)?;
    let module_major = manifest.module_major;
    let cue_language = parse_v_version(&manifest.cue_language_version, "cue_language_version")?;
    let supported_language =
        parse_v_version(CUEENGINE_LANGUAGE_VERSION, "cuengine language version")?;
    if cue_language > supported_language {
        bail!(
            "cue_language_version {} is newer than cuengine's embedded CUE {}; lower it or upgrade cuengine",
            manifest.cue_language_version,
            CUEENGINE_LANGUAGE_VERSION
        );
    }
    parse_plain_version(&manifest.terraform_cli_version, "terraform_cli_version")?;

    if manifest.providers.is_empty() {
        bail!("providers.cue must list at least one Terraform provider");
    }
    for (path, provider) in &manifest.providers {
        validate_path(path, "provider path")?;
        validate_source(&provider.source)?;
        if provider.versions.is_empty() {
            bail!("provider {path:?} needs at least one pinned version");
        }
        for (provider_version, release) in &provider.versions {
            parse_plain_version(provider_version, "Terraform provider version")?;
            let path_version = release.path_version.as_deref().unwrap_or(provider_version);
            validate_module_segment(path_version).with_context(|| {
                format!(
                    "{path}@{provider_version} has invalid path_version {path_version:?}; set a lowercase module path segment"
                )
            })?;
            let module_path = format!(
                "{}/{}/{}@v{}",
                manifest.module_prefix, path, path_version, module_major
            );
            if module_path.len() > 128 {
                bail!("CUE module path is longer than 128 characters: {module_path}");
            }
        }
    }
    releases(manifest)?;
    Ok(())
}

fn validate_module_prefix(prefix: &str) -> Result<()> {
    validate_path(prefix, "module_prefix")?;
    let host = prefix.split('/').next().unwrap_or_default();
    if !host.contains('.') {
        bail!("module_prefix must begin with a domain name, got {prefix:?}");
    }
    Ok(())
}

fn validate_source(source: &str) -> Result<()> {
    let parts: Vec<_> = source.split('/').collect();
    if !matches!(parts.len(), 2 | 3) || parts.iter().any(|part| part.is_empty()) {
        bail!("Terraform source {source:?} must be namespace/type or hostname/namespace/type");
    }
    for part in &parts {
        if !part.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-_".contains(&byte)
        }) {
            bail!("Terraform source {source:?} contains an invalid path element");
        }
    }
    Ok(())
}

fn validate_path(path: &str, description: &str) -> Result<()> {
    if path.is_empty() {
        bail!("{description} must not be empty");
    }
    for part in path.split('/') {
        validate_module_segment(part).with_context(|| format!("invalid {description} {path:?}"))?;
    }
    Ok(())
}

fn validate_module_segment(part: &str) -> Result<()> {
    let valid_chars = part
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte));
    let starts_alphanumeric = part
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric);
    let periods_ok = !part.contains("..") && !part.contains("._") && !part.contains("_.");
    let underscores_ok = !part.contains("___");
    if !valid_chars || !starts_alphanumeric || !periods_ok || !underscores_ok {
        bail!("not a valid lowercase CUE module path element: {part:?}");
    }
    Ok(())
}

fn parse_plain_version(value: &str, field: &str) -> Result<Version> {
    Version::parse(value).with_context(|| format!("{field} must be an exact semantic version"))
}

fn parse_v_version(value: &str, field: &str) -> Result<Version> {
    let version = value
        .strip_prefix('v')
        .with_context(|| format!("{field} must start with v"))?;
    parse_plain_version(version, field)
}
