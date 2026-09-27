use std::fs;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tempfile::tempdir;

use crate::util::command_output;

pub struct ProviderSchema {
    pub schema_key: String,
    pub schema: Value,
    pub raw_schema: String,
    pub lockfile: String,
}

pub fn provider_schema(source: &str, version: &str) -> Result<ProviderSchema> {
    let local_name = "providertarget";
    let quoted_source = serde_json::to_string(source)?;
    let quoted_version = serde_json::to_string(&format!("= {version}"))?;
    let quoted_local_name = serde_json::to_string(local_name)?;
    let config = format!(
        "terraform {{\n  required_providers {{\n    {local_name} = {{\n      source  = {quoted_source}\n      version = {quoted_version}\n    }}\n  }}\n}}\n\nprovider {quoted_local_name} {{}}\n"
    );

    let temporary = tempdir().context("create temporary Terraform working directory")?;
    let workdir = temporary.path();
    fs::write(workdir.join("provider.tf"), config).context("write temporary provider.tf")?;
    command_output(
        "terraform",
        &[
            "init".to_owned(),
            "-backend=false".to_owned(),
            "-input=false".to_owned(),
            "-no-color".to_owned(),
        ],
        Some(workdir),
    )?;
    let raw_schema = command_output(
        "terraform",
        &[
            "providers".to_owned(),
            "schema".to_owned(),
            "-json".to_owned(),
        ],
        Some(workdir),
    )?;
    let lockfile = fs::read_to_string(workdir.join(".terraform.lock.hcl"))
        .context("terraform init did not create .terraform.lock.hcl")?;
    let document: Value =
        serde_json::from_str(&raw_schema).context("parse Terraform schema JSON")?;
    let format_version = document
        .get("format_version")
        .and_then(Value::as_str)
        .context("Terraform schema JSON is missing format_version")?;
    if !format_version.starts_with("1.") {
        bail!("unsupported Terraform provider schema format {format_version:?}");
    }

    let schema_key = if source.matches('/').count() == 1 {
        format!("registry.terraform.io/{source}")
    } else {
        source.to_owned()
    };
    let schema = document
        .get("provider_schemas")
        .and_then(Value::as_object)
        .and_then(|schemas| schemas.get(&schema_key))
        .cloned()
        .with_context(|| {
            let available = document
                .get("provider_schemas")
                .and_then(Value::as_object)
                .map(|schemas| schemas.keys().cloned().collect::<Vec<_>>().join(", "))
                .filter(|keys| !keys.is_empty())
                .unwrap_or_else(|| "none".to_owned());
            format!("Terraform did not return {schema_key:?}; providers returned: {available}")
        })?;
    Ok(ProviderSchema {
        schema_key,
        schema,
        raw_schema,
        lockfile,
    })
}

pub fn check_version(expected: &str) -> Result<()> {
    let output = command_output(
        "terraform",
        &["version".to_owned(), "-json".to_owned()],
        None,
    )?;
    let document: Value = serde_json::from_str(&output).context("parse terraform version -json")?;
    let actual = document
        .get("terraform_version")
        .and_then(Value::as_str)
        .context("terraform version -json did not include terraform_version")?;
    if actual != expected {
        bail!("providers.cue pins Terraform CLI {expected}, but installed version is {actual}");
    }
    Ok(())
}
