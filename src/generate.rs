use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cuengine::evaluate_cue_package;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tempfile::NamedTempFile;

use crate::cue::registry as cue_registry;
use crate::manifest::{self, ProviderRelease};
use crate::registry;
use crate::registry::http;
use crate::schema;
use crate::terraform;
use crate::util::{json_string, sha256};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReleaseState {
    #[serde(default)]
    completed: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pending: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    published: BTreeMap<String, BTreeMap<String, String>>,
}

pub fn run(
    root: &Path,
    provider_filter: Option<&str>,
    version_filter: Option<&str>,
    output: &Path,
    snapshots: &Path,
) -> Result<()> {
    let manifest = manifest::load(root)?;
    terraform::check_version(&manifest.terraform_cli_version)?;

    let releases = if let Some(version) = version_filter {
        let provider = provider_filter.context("--provider-version requires --provider")?;
        vec![manifest::release(&manifest, provider, version)?]
    } else {
        let mut releases = manifest::releases(&manifest)?;
        if let Some(provider) = provider_filter {
            releases.retain(|release| release.provider_path == provider);
        }
        releases
    };
    if releases.is_empty() {
        bail!("no configured provider releases matched the selection");
    }

    let output = rooted_path(root, output);
    let snapshots = rooted_path(root, snapshots);
    let generator_hash = generator_fingerprint(root)?;
    for release in releases {
        generate_release(&manifest, &release, &output, &snapshots, &generator_hash)?;
    }
    Ok(())
}

pub fn registry_tags(root: &Path) -> Result<()> {
    let manifest = manifest::load(root)?;
    let release = manifest::releases(&manifest)?
        .into_iter()
        .next()
        .context("providers.cue does not configure any provider releases")?;
    let username = std::env::var("GHCR_USERNAME").unwrap_or_default();
    let password = std::env::var("GHCR_TOKEN").unwrap_or_default();
    if username.is_empty() != password.is_empty() {
        bail!("GHCR_USERNAME and GHCR_TOKEN must either both be set or both be empty");
    }
    let client = http::client()?;
    let tags = cue_registry::published_tags(&client, &release.module_path, &username, &password)?;
    let mut tags = tags.into_iter().collect::<Vec<_>>();
    tags.sort();
    println!("{}", serde_json::to_string(&tags)?);
    Ok(())
}

pub fn update(
    root: &Path,
    provider_filter: Option<&str>,
    output: &Path,
    snapshots: &Path,
    state_file: &Path,
    published_tags_file: Option<&Path>,
    dry_run: bool,
    defer_publish: bool,
) -> Result<()> {
    let manifest = manifest::load(root)?;
    let output = rooted_path(root, output);
    let snapshots = rooted_path(root, snapshots);
    let state_file = rooted_path(root, state_file);
    let mut state = load_state(&state_file)?;
    let published_tags = published_tags_file
        .map(read_published_tags)
        .transpose()?
        .unwrap_or_default();

    let mut candidates = manifest::releases(&manifest)?;
    if let Some(provider) = provider_filter {
        candidates.retain(|release| release.provider_path == provider);
        if candidates.is_empty() {
            bail!("provider {provider:?} is not configured in providers.cue");
        }
    }
    candidates.extend(pending_releases(&manifest, &state, provider_filter)?);
    candidates.extend(published_releases(&manifest, &state, provider_filter)?);
    let client = http::client()?;
    candidates.extend(registry::newer_releases(
        &client,
        &manifest,
        provider_filter,
    )?);
    candidates.sort_by(|left, right| {
        left.provider_path
            .cmp(&right.provider_path)
            .then_with(|| left.provider_version.cmp(&right.provider_version))
    });
    candidates.dedup_by(|left, right| {
        left.provider_path == right.provider_path && left.provider_version == right.provider_version
    });

    let generator_hash = generator_fingerprint(root)?;

    let mut pending = Vec::new();
    let mut already_complete = Vec::new();
    for release in candidates {
        let tag = cue_registry::registry_tag(&release.module_path, &release.module_version)?;
        if published_tags_file.is_some() {
            if published_tags.contains(&tag) {
                if !dry_run {
                    state_mark_published(&mut state, &release);
                }
                continue;
            }
            state_forget_published(&mut state, &release);
        } else if state_is_published(&state, &release) {
            continue;
        }
        if published_tags_file.is_none() && state_contains(&state, &release, &generator_hash) {
            continue;
        }
        if release_is_complete(&manifest, &release, &output, &snapshots, &generator_hash)? {
            println!(
                "already generated {}@{}",
                release.provider_path, release.provider_version
            );
            already_complete.push(release);
        } else {
            pending.push(release);
        }
    }

    if dry_run {
        let publishable = if defer_publish {
            already_complete.len()
        } else {
            0
        };
        if pending.is_empty() && publishable == 0 {
            println!("up to date: no new provider releases");
        } else {
            if !pending.is_empty() {
                println!("would generate {} provider release(s)", pending.len());
            }
            for release in pending {
                println!("{}@{}", release.provider_path, release.provider_version);
            }
            if publishable > 0 {
                println!("would publish {publishable} previously generated provider release(s)");
                for release in already_complete {
                    println!("{}@{}", release.provider_path, release.provider_version);
                }
            }
        }
        return Ok(());
    }

    for release in &pending {
        state_pending_insert(&mut state, release);
    }
    for release in &already_complete {
        if defer_publish {
            state_pending_insert(&mut state, release);
        } else {
            state_insert(&mut state, release, &generator_hash);
        }
    }
    write_state(&state_file, &state)?;

    if pending.is_empty() {
        println!("up to date: no new provider releases");
        return Ok(());
    }

    terraform::check_version(&manifest.terraform_cli_version)?;
    let pending_count = pending.len();
    let mut generated_count = 0;
    let mut failures = Vec::new();
    for release in pending {
        match generate_release(&manifest, &release, &output, &snapshots, &generator_hash) {
            Ok(()) => {
                if defer_publish {
                    state_pending_insert(&mut state, &release);
                } else {
                    state_insert(&mut state, &release, &generator_hash);
                }
                generated_count += 1;
            }
            Err(error) => {
                let message = format!(
                    "{}@{}: {error:#}",
                    release.provider_path, release.provider_version
                );
                eprintln!("failed to generate {message}");
                failures.push(message);
            }
        }
    }

    for release in already_complete {
        if defer_publish {
            state_pending_insert(&mut state, &release);
        } else {
            state_insert(&mut state, &release, &generator_hash);
        }
    }
    write_state(&state_file, &state)?;
    println!("generated {generated_count} of {pending_count} new provider release(s)");
    if !failures.is_empty() {
        bail!(
            "failed to generate {} of {pending_count} provider release(s): {}",
            failures.len(),
            failures.join("; ")
        );
    }
    Ok(())
}

pub fn publish_generated(
    root: &Path,
    output: &Path,
    snapshots: &Path,
    state_file: &Path,
) -> Result<()> {
    require_serialized_publisher()?;
    let manifest = manifest::load(root)?;
    let output = rooted_path(root, output);
    let snapshots = rooted_path(root, snapshots);
    let state_file = rooted_path(root, state_file);
    let mut state = load_state(&state_file)?;
    let pending = pending_releases(&manifest, &state, None)?;
    if pending.is_empty() {
        println!("up to date: no generated provider releases are pending publication");
        return Ok(());
    }

    let generator_hash = generator_fingerprint(root)?;
    let mut candidates = Vec::new();
    let mut failures = Vec::new();
    for release in pending {
        match release_is_complete(&manifest, &release, &output, &snapshots, &generator_hash) {
            Ok(true) => candidates.push(release),
            Ok(false) => println!(
                "{}@{} has no valid generated artifact; leaving it pending",
                release.provider_path, release.provider_version
            ),
            Err(error) => {
                let message = format!(
                    "{}@{}: {error:#}",
                    release.provider_path, release.provider_version
                );
                eprintln!("failed to validate generated {message}");
                failures.push(message);
            }
        }
    }
    if candidates.is_empty() {
        if failures.is_empty() {
            println!("no valid generated provider releases are ready to publish");
            return Ok(());
        }
        bail!(
            "failed to validate {} generated provider release(s): {}",
            failures.len(),
            failures.join("; ")
        );
    }

    let username =
        std::env::var("GHCR_USERNAME").context("GHCR_USERNAME is required to publish")?;
    let password = std::env::var("GHCR_TOKEN").context("GHCR_TOKEN is required to publish")?;
    let client = http::client()?;
    let ghcr_registry = cue_registry::GhcrRegistry::connect(
        &client,
        &candidates[0].module_path,
        &username,
        &password,
    )?;
    let published_tags = ghcr_registry.tags()?;

    let registry_config = root.join("cue-registry.cue");
    let mut published_count = 0;
    let mut reconciled_count = 0;
    for release in candidates {
        let tag = cue_registry::registry_tag(&release.module_path, &release.module_version)?;
        if published_tags.contains(&tag) {
            println!("already published {tag} for {}", release.module_path);
            state_insert(&mut state, &release, &generator_hash);
            state_mark_published(&mut state, &release);
            write_state(&state_file, &state)?;
            reconciled_count += 1;
            continue;
        }

        let result = verify_registry_target(&release, &registry_config).and_then(|()| {
            if ghcr_registry.tag_exists(&tag)? {
                return Ok(false);
            }
            publish_module(&module_root(&output, &release), &release, &registry_config)?;
            Ok(true)
        });
        match result {
            Ok(published) => {
                state_insert(&mut state, &release, &generator_hash);
                state_mark_published(&mut state, &release);
                write_state(&state_file, &state)?;
                if published {
                    published_count += 1;
                } else {
                    reconciled_count += 1;
                }
            }
            Err(error) => {
                let message = format!(
                    "{}@{}: {error:#}",
                    release.provider_path, release.provider_version
                );
                eprintln!("failed to publish {message}");
                failures.push(message);
            }
        }
    }

    println!(
        "published {published_count} and reconciled {reconciled_count} provider release(s) in GHCR"
    );
    if !failures.is_empty() {
        bail!(
            "failed to publish {} provider release(s): {}",
            failures.len(),
            failures.join("; ")
        );
    }
    Ok(())
}

fn require_serialized_publisher() -> Result<()> {
    let actions = std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true");
    let main_branch = std::env::var("GITHUB_REF").is_ok_and(|value| value == "refs/heads/main");
    if !actions || !main_branch {
        bail!("GHCR publication is restricted to the serialized GitHub Actions workflow on main");
    }
    Ok(())
}

fn pending_releases(
    manifest: &manifest::Manifest,
    state: &ReleaseState,
    provider_filter: Option<&str>,
) -> Result<Vec<ProviderRelease>> {
    let mut releases = Vec::new();
    for (provider_path, versions) in &state.pending {
        if provider_filter.is_some_and(|filter| filter != provider_path)
            || !manifest.providers.contains_key(provider_path)
        {
            continue;
        }
        for version in versions {
            releases.push(manifest::release(manifest, provider_path, version)?);
        }
    }
    Ok(releases)
}

fn published_releases(
    manifest: &manifest::Manifest,
    state: &ReleaseState,
    provider_filter: Option<&str>,
) -> Result<Vec<ProviderRelease>> {
    let mut releases = Vec::new();
    for (provider_path, versions) in &state.published {
        if provider_filter.is_some_and(|filter| filter != provider_path)
            || !manifest.providers.contains_key(provider_path)
        {
            continue;
        }
        for version in versions.keys() {
            releases.push(manifest::release(manifest, provider_path, version)?);
        }
    }
    Ok(releases)
}

fn state_pending_insert(state: &mut ReleaseState, release: &ProviderRelease) {
    state
        .pending
        .entry(release.provider_path.clone())
        .or_default()
        .insert(release.provider_version.clone());
}

fn forget_pending(state: &mut ReleaseState, release: &ProviderRelease) {
    if let Some(versions) = state.pending.get_mut(&release.provider_path) {
        versions.remove(&release.provider_version);
        if versions.is_empty() {
            state.pending.remove(&release.provider_path);
        }
    }
}

fn verify_registry_target(release: &ProviderRelease, registry_config: &Path) -> Result<()> {
    let module_path = release
        .module_path
        .rsplit_once("@v")
        .map(|(path, _)| path)
        .context("CUE module path is missing its major version")?;
    let module_reference = format!("{module_path}@{}", release.module_version);
    let output = Command::new("cue")
        .args(["mod", "resolve", module_reference.as_str()])
        .env(
            "CUE_REGISTRY",
            format!("file:{}", registry_config.display()),
        )
        .output()
        .with_context(|| format!("resolve CUE registry destination for {module_reference}"))?;
    if !output.status.success() {
        bail!(
            "`cue mod resolve {module_reference}` failed: {}{}{}",
            String::from_utf8_lossy(&output.stdout).trim(),
            if output.stdout.is_empty() || output.stderr.is_empty() {
                ""
            } else {
                "\n"
            },
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let expected = format!(
        "ghcr.io/cuenv/terraform-cue:{}",
        cue_registry::registry_tag(&release.module_path, &release.module_version)?
    );
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if actual != expected {
        bail!(
            "CUE resolves {module_reference} to {actual:?}, expected GHCR destination {expected:?}"
        );
    }
    Ok(())
}

fn publish_module(
    module_root: &Path,
    release: &ProviderRelease,
    registry_config: &Path,
) -> Result<()> {
    let tag = &release.module_version;
    println!("publishing {}@{tag}", release.module_path);
    let output = Command::new("cue")
        .args(["mod", "publish", tag.as_str()])
        .current_dir(module_root)
        .env(
            "CUE_REGISTRY",
            format!("file:{}", registry_config.display()),
        )
        .output()
        .with_context(|| format!("start `cue mod publish {tag}` for {}", release.module_path))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "`cue mod publish {tag}` failed for {} ({}): {}{}{}",
            release.module_path,
            output.status,
            stdout.trim(),
            if stdout.is_empty() || stderr.is_empty() {
                ""
            } else {
                "\n"
            },
            stderr.trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        print!("{}", stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        eprint!("{}", stderr);
    }
    Ok(())
}

fn generate_release(
    manifest: &manifest::Manifest,
    release: &ProviderRelease,
    output: &Path,
    snapshots: &Path,
    generator_hash: &str,
) -> Result<()> {
    let provider_schema = terraform::provider_schema(&release.source, &release.provider_version)
        .with_context(|| {
            format!(
                "fetch schema for {}@{}",
                release.provider_path, release.provider_version
            )
        })?;
    let schema_hash = sha256(provider_schema.raw_schema.as_bytes());
    let (cue_source, counts) = schema::render_provider(
        &release.provider_path,
        &release.source,
        &release.provider_version,
        &provider_schema.schema,
        &schema_hash,
    )?;

    let module_root = module_root(output, release);
    let cue_module = module_cue(release, &manifest.cue_language_version)?;
    fs::create_dir_all(module_root.join("cue.mod"))
        .with_context(|| format!("create module root {}", module_root.display()))?;
    fs::write(module_root.join("cue.mod/module.cue"), cue_module)
        .context("write generated cue.mod/module.cue")?;
    fs::write(module_root.join("schema.cue"), cue_source).context("write generated schema.cue")?;
    evaluate_cue_package(&module_root, &schema::package_name(&release.provider_path))
        .with_context(|| {
            format!(
                "validate generated CUE definitions for {}",
                release.module_path
            )
        })?;

    let snapshot_root = snapshot_root(snapshots, release);
    fs::create_dir_all(&snapshot_root).context("create provider schema snapshot directory")?;
    fs::write(
        snapshot_root.join("terraform-providers-schema.json"),
        &provider_schema.raw_schema,
    )
    .context("write raw Terraform provider schema")?;
    fs::write(
        snapshot_root.join("terraform.lock.hcl"),
        &provider_schema.lockfile,
    )
    .context("write Terraform provider lockfile")?;
    let metadata = json!({
        "terraform_provider": {
            "source": release.source,
            "version": release.provider_version,
            "schema_key": provider_schema.schema_key,
        },
        "cue_module": {
            "path": release.module_path,
            "version": release.module_version,
            "language_version": manifest.cue_language_version,
        },
        "generation": {
            "terraform_cli_version": manifest.terraform_cli_version,
            "generator_sha256": generator_hash,
            "schema_sha256": schema_hash,
            "module_file_sha256": sha256(&fs::read(module_root.join("cue.mod/module.cue"))?),
            "schema_file_sha256": sha256(&fs::read(module_root.join("schema.cue"))?),
            "lockfile_sha256": sha256(provider_schema.lockfile.as_bytes()),
            "schema_categories": counts,
        },
    });
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)
        .context("serialize generation metadata")?
        .into_iter()
        .chain([b'\n'])
        .collect::<Vec<_>>();
    let mut temporary =
        NamedTempFile::new_in(&snapshot_root).context("stage provider generation metadata")?;
    temporary
        .write_all(&metadata_bytes)
        .context("write staged provider generation metadata")?;
    temporary
        .persist(snapshot_root.join("metadata.json"))
        .context("write generation metadata")?;

    println!(
        "generated {} from {}@{}",
        module_root.display(),
        release.source,
        release.provider_version
    );
    Ok(())
}

fn release_is_complete(
    manifest: &manifest::Manifest,
    release: &ProviderRelease,
    output: &Path,
    snapshots: &Path,
    generator_hash: &str,
) -> Result<bool> {
    let module_root = module_root(output, release);
    let snapshot_root = snapshot_root(snapshots, release);
    let metadata_path = snapshot_root.join("metadata.json");
    if !metadata_path.exists() {
        return Ok(false);
    }

    let module_file = module_root.join("cue.mod/module.cue");
    let schema_file = module_root.join("schema.cue");
    let raw_schema_file = snapshot_root.join("terraform-providers-schema.json");
    let lockfile = snapshot_root.join("terraform.lock.hcl");
    for path in [&module_file, &schema_file, &raw_schema_file, &lockfile] {
        if !path.is_file() {
            bail!(
                "{} has completion metadata but is missing {}; regenerate that pinned release with `terraform-cue generate`",
                release.module_path,
                path.display()
            );
        }
    }

    let metadata: Value = serde_json::from_slice(
        &fs::read(&metadata_path).context("read provider generation metadata")?,
    )
    .with_context(|| format!("parse provider metadata at {}", metadata_path.display()))?;
    let actual_module =
        fs::read_to_string(&module_file).context("read generated CUE module identity")?;
    let expected_module = module_cue(release, &manifest.cue_language_version)?;
    if actual_module != expected_module {
        bail!(
            "{} has a cue.mod/module.cue identity that does not match its expected module path and language version",
            release.module_path
        );
    }
    if metadata_value(&metadata, "generation.generator_sha256") != Some(generator_hash) {
        return Ok(false);
    }
    for (path, expected) in [
        ("terraform_provider.source", release.source.as_str()),
        (
            "terraform_provider.version",
            release.provider_version.as_str(),
        ),
        ("cue_module.path", release.module_path.as_str()),
        ("cue_module.version", release.module_version.as_str()),
        (
            "cue_module.language_version",
            manifest.cue_language_version.as_str(),
        ),
        (
            "generation.terraform_cli_version",
            manifest.terraform_cli_version.as_str(),
        ),
    ] {
        if metadata_value(&metadata, path) != Some(expected) {
            bail!(
                "{} has metadata that does not match the configured {} ({expected:?}); regenerate explicitly with `terraform-cue generate`",
                release.module_path,
                path
            );
        }
    }

    for (field, path) in [
        ("module_file_sha256", &module_file),
        ("schema_file_sha256", &schema_file),
        ("schema_sha256", &raw_schema_file),
        ("lockfile_sha256", &lockfile),
    ] {
        let expected = metadata
            .get("generation")
            .and_then(Value::as_object)
            .and_then(|generation| generation.get(field))
            .and_then(Value::as_str)
            .with_context(|| format!("metadata is missing generation.{field}"))?;
        let actual = sha256(&fs::read(path).with_context(|| format!("read {}", path.display()))?);
        if actual != expected {
            bail!(
                "{} failed its {} integrity check; regenerate explicitly with `terraform-cue generate`",
                release.module_path,
                field
            );
        }
    }
    Ok(true)
}

fn metadata_value<'a>(metadata: &'a Value, path: &str) -> Option<&'a str> {
    let mut value = metadata;
    for key in path.split('.') {
        value = value.get(key)?;
    }
    value.as_str()
}

fn load_state(path: &Path) -> Result<ReleaseState> {
    if !path.exists() {
        return Ok(ReleaseState::default());
    }
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse provider release state at {}", path.display()))
}

fn read_published_tags(path: &Path) -> Result<HashSet<String>> {
    let tags: Vec<String> = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("read GHCR tags from {}", path.display()))?,
    )
    .with_context(|| format!("parse GHCR tags from {}", path.display()))?;
    Ok(tags.into_iter().collect())
}

fn write_state(path: &Path, state: &ReleaseState) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let bytes = serde_json::to_vec_pretty(state)
        .context("serialize provider release state")?
        .into_iter()
        .chain([b'\n'])
        .collect::<Vec<_>>();
    let mut temporary = NamedTempFile::new_in(parent).context("stage provider release state")?;
    temporary
        .write_all(&bytes)
        .context("write staged provider release state")?;
    temporary
        .persist(path)
        .with_context(|| format!("write provider release state to {}", path.display()))?;
    Ok(())
}

fn state_contains(state: &ReleaseState, release: &ProviderRelease, generator_hash: &str) -> bool {
    let identity = release_identity(release, generator_hash);
    state
        .completed
        .get(&release.provider_path)
        .is_some_and(|versions| versions.get(&release.provider_version) == Some(&identity))
}

fn state_insert(state: &mut ReleaseState, release: &ProviderRelease, generator_hash: &str) {
    forget_pending(state, release);
    state
        .completed
        .entry(release.provider_path.clone())
        .or_default()
        .insert(
            release.provider_version.clone(),
            release_identity(release, generator_hash),
        );
}

fn state_is_published(state: &ReleaseState, release: &ProviderRelease) -> bool {
    let identity = published_identity(release);
    state
        .published
        .get(&release.provider_path)
        .is_some_and(|versions| versions.get(&release.provider_version) == Some(&identity))
}

fn state_mark_published(state: &mut ReleaseState, release: &ProviderRelease) {
    forget_pending(state, release);
    state
        .published
        .entry(release.provider_path.clone())
        .or_default()
        .insert(
            release.provider_version.clone(),
            published_identity(release),
        );
}

fn state_forget_published(state: &mut ReleaseState, release: &ProviderRelease) {
    if let Some(versions) = state.published.get_mut(&release.provider_path) {
        versions.remove(&release.provider_version);
        if versions.is_empty() {
            state.published.remove(&release.provider_path);
        }
    }
}

fn published_identity(release: &ProviderRelease) -> String {
    format!(
        "{}|{}|{}",
        release.source, release.module_path, release.module_version
    )
}

fn release_identity(release: &ProviderRelease, generator_hash: &str) -> String {
    format!(
        "{}|{}|{}|{}",
        release.source, release.module_path, release.module_version, generator_hash
    )
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{ReleaseState, pending_releases};
    use crate::manifest::{Manifest, Provider};

    #[test]
    fn restores_pending_versions_outside_the_discovery_window() {
        let provider_path = "terraform/cloudflare/cloudflare";
        let manifest = Manifest {
            module_prefix: "ghcr.io/cuenv".to_owned(),
            cue_language_version: "v0.16.0".to_owned(),
            terraform_cli_version: "1.16.4".to_owned(),
            providers: BTreeMap::from([(
                provider_path.to_owned(),
                Provider {
                    source: "cloudflare/cloudflare".to_owned(),
                    minimum_version: "5.26.0".to_owned(),
                },
            )]),
        };
        let state = ReleaseState {
            completed: BTreeMap::new(),
            pending: BTreeMap::from([(
                provider_path.to_owned(),
                BTreeSet::from(["5.25.0".to_owned()]),
            )]),
            published: BTreeMap::new(),
        };

        let releases =
            pending_releases(&manifest, &state, None).expect("restore pending provider releases");

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].provider_version, "5.25.0");
    }
}

fn generator_fingerprint(root: &Path) -> Result<String> {
    let mut sources = vec![root.join("Cargo.toml"), root.join("Cargo.lock")];
    let mut modules = Vec::new();
    collect_rust_sources(&root.join("src"), &mut modules)?;
    modules.sort();
    sources.extend(modules);

    let mut fingerprint = Vec::new();
    for source in sources {
        let relative = source
            .strip_prefix(root)
            .context("generator source is outside the repository")?;
        fingerprint.extend_from_slice(relative.to_string_lossy().as_bytes());
        fingerprint.push(0);
        fingerprint.extend_from_slice(
            &fs::read(&source).with_context(|| format!("read {}", source.display()))?,
        );
        fingerprint.push(0);
    }
    Ok(sha256(&fingerprint))
}

fn collect_rust_sources(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory).context("list generator source files")? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_sources(&path, sources)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    Ok(())
}

fn module_root(output: &Path, release: &ProviderRelease) -> PathBuf {
    output
        .join(&release.provider_path)
        .join(local_version(release))
}

fn snapshot_root(snapshots: &Path, release: &ProviderRelease) -> PathBuf {
    snapshots
        .join(&release.provider_path)
        .join(local_version(release))
}

fn local_version(release: &ProviderRelease) -> &str {
    release
        .module_version
        .strip_prefix('v')
        .unwrap_or(&release.module_version)
}

fn rooted_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn module_cue(release: &ProviderRelease, language_version: &str) -> Result<String> {
    Ok(format!(
        "module: {}\nlanguage: version: {}\nsource: kind: \"self\"\n",
        json_string(&release.module_path)?,
        json_string(language_version)?
    ))
}
