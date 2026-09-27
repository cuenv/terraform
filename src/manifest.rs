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
    pub cue_language_version: String,
    pub terraform_cli_version: String,
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Debug, Deserialize)]
pub struct Provider {
    pub source: String,
    #[serde(rename = "minimumVersion")]
    pub minimum_version: String,
}

#[derive(Clone, Debug)]
pub struct ProviderRelease {
    pub provider_path: String,
    pub source: String,
    pub provider_version: String,
    pub module_path: String,
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
        let generated = release(manifest, provider_path, &provider.minimum_version)?;
        let module_path = generated.module_path.clone();
        if !module_paths.insert(module_path.clone()) {
            bail!("duplicate CUE module path {module_path:?}");
        }
        result.push(generated);
    }
    Ok(result)
}

pub fn release(
    manifest: &Manifest,
    provider_path: &str,
    provider_version: &str,
) -> Result<ProviderRelease> {
    let version = parse_plain_version(provider_version, "Terraform provider version")?;
    let provider = manifest
        .providers
        .get(provider_path)
        .with_context(|| format!("unknown provider path {provider_path:?}"))?;
    let module_path = format!(
        "{}/{}@v{}",
        manifest.module_prefix, provider_path, version.major
    );
    if module_path.len() > 128 {
        bail!("CUE module path is longer than 128 characters: {module_path}");
    }
    Ok(ProviderRelease {
        provider_path: provider_path.to_owned(),
        source: provider.source.clone(),
        provider_version: provider_version.to_owned(),
        module_path,
    })
}

fn validate(manifest: &Manifest) -> Result<()> {
    validate_module_prefix(&manifest.module_prefix)?;
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
        let minimum_version = parse_plain_version(&provider.minimum_version, "minimumVersion")?;
        if !minimum_version.pre.is_empty() || !minimum_version.build.is_empty() {
            bail!("minimumVersion for provider {path:?} must be a stable semantic version");
        }
        let module_path = format!(
            "{}/{}@v{}",
            manifest.module_prefix, path, minimum_version.major
        );
        if module_path.len() > 128 {
            bail!("CUE module path is longer than 128 characters: {module_path}");
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Manifest, Provider, release};

    #[test]
    fn module_major_and_release_tag_follow_provider_version() {
        let manifest = Manifest {
            module_prefix: "github.com/cuenv/terraform".to_owned(),
            cue_language_version: "v0.16.0".to_owned(),
            terraform_cli_version: "1.16.4".to_owned(),
            providers: BTreeMap::from([(
                "terraform/cloudflare/cloudflare".to_owned(),
                Provider {
                    source: "cloudflare/cloudflare".to_owned(),
                    minimum_version: "5.26.0".to_owned(),
                },
            )]),
        };

        let version_5 = release(&manifest, "terraform/cloudflare/cloudflare", "5.26.0")
            .expect("Terraform provider release");
        let version_6 = release(&manifest, "terraform/cloudflare/cloudflare", "6.0.0")
            .expect("Terraform provider release");

        assert_eq!(
            version_5.module_path,
            "github.com/cuenv/terraform/terraform/cloudflare/cloudflare@v5"
        );
        assert_eq!(
            version_6.module_path,
            "github.com/cuenv/terraform/terraform/cloudflare/cloudflare@v6"
        );
        assert_eq!(version_5.provider_version, "5.26.0");
        assert_eq!(version_6.provider_version, "6.0.0");
    }
}
