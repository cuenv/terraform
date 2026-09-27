use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cuengine::evaluate_cue_package;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tempfile::NamedTempFile;

use crate::manifest::{self, ProviderRelease};
use crate::registry;
use crate::registry_http;
use crate::schema;
use crate::terraform;
use crate::util::{json_string, sha256};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReleaseState {
    #[serde(default)]
    completed: BTreeMap<String, BTreeMap<String, String>>,
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

pub fn update(
    root: &Path,
    provider_filter: Option<&str>,
    output: &Path,
    snapshots: &Path,
    state_file: &Path,
    dry_run: bool,
) -> Result<()> {
    let manifest = manifest::load(root)?;
    let output = rooted_path(root, output);
    let snapshots = rooted_path(root, snapshots);
    let state_file = rooted_path(root, state_file);

    let mut candidates = manifest::releases(&manifest)?;
    if let Some(provider) = provider_filter {
        candidates.retain(|release| release.provider_path == provider);
        if candidates.is_empty() {
            bail!("provider {provider:?} is not configured in providers.cue");
        }
    }
    let client = registry_http::client()?;
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

    let mut state = load_state(&state_file)?;
    let mut pending = Vec::new();
    let mut already_complete = Vec::new();
    for release in candidates {
        if state_contains(&state, &release) {
            continue;
        }
        if release_is_complete(&manifest, &release, &output, &snapshots)? {
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
        if pending.is_empty() {
            println!("up to date: no new provider releases");
        } else {
            println!("would generate {} provider release(s)", pending.len());
            for release in pending {
                println!("{}@{}", release.provider_path, release.provider_version);
            }
        }
        return Ok(());
    }

    if pending.is_empty() {
        for release in already_complete {
            state_insert(&mut state, &release);
        }
        write_state(&state_file, &state)?;
        println!("up to date: no new provider releases");
        return Ok(());
    }

    terraform::check_version(&manifest.terraform_cli_version)?;
    let generator_hash = generator_fingerprint(root)?;
    let pending_count = pending.len();
    let mut generated_count = 0;
    let mut failures = Vec::new();
    for release in pending {
        match generate_release(&manifest, &release, &output, &snapshots, &generator_hash) {
            Ok(()) => {
                state_insert(&mut state, &release);
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
        state_insert(&mut state, &release);
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
    for (path, expected) in [
        ("terraform_provider.source", release.source.as_str()),
        (
            "terraform_provider.version",
            release.provider_version.as_str(),
        ),
        ("cue_module.path", release.module_path.as_str()),
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

fn state_contains(state: &ReleaseState, release: &ProviderRelease) -> bool {
    state
        .completed
        .get(&release.provider_path)
        .is_some_and(|versions| versions.get(&release.provider_version) == Some(&release.source))
}

fn state_insert(state: &mut ReleaseState, release: &ProviderRelease) {
    state
        .completed
        .entry(release.provider_path.clone())
        .or_default()
        .insert(release.provider_version.clone(), release.source.clone());
}

fn generator_fingerprint(root: &Path) -> Result<String> {
    let mut sources = vec![root.join("Cargo.toml"), root.join("Cargo.lock")];
    let mut modules = fs::read_dir(root.join("src"))
        .context("list generator source files")?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    modules.retain(|path| path.extension().is_some_and(|extension| extension == "rs"));
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

fn module_root(output: &Path, release: &ProviderRelease) -> PathBuf {
    output
        .join(&release.provider_path)
        .join(&release.path_version)
}

fn snapshot_root(snapshots: &Path, release: &ProviderRelease) -> PathBuf {
    snapshots
        .join(&release.provider_path)
        .join(&release.path_version)
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
