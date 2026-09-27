# pulumi-cue

Fetch Terraform provider schemas and generate versioned CUE definition files.

## Provider manifest

`providers.cue` maps a stable provider path to its Terraform Registry source and one or more starting releases. For example, `cloudflare/cloudflare` version `5.22.0` generates `generated/terraform/cloudflare/5.22.0/schema.cue`.

```cue
"terraform/cloudflare": {
    source: "cloudflare/cloudflare"
    versions: {"5.22.0": {}}
}
```

For automatic release discovery, the highest version listed for each provider is the starting point. The updater considers later stable semantic versions published in the last 30 days, using Terraform Registry publication timestamps. Versions listed explicitly in this manifest bypass the time filter. Add a provider's current release as its starting point to avoid backfilling its older history. Automatic release discovery currently supports the public `registry.terraform.io`; providers from custom registries can still be generated manually from pinned versions.

## Requirements

- Rust stable, Go 1.25 or newer, and a C toolchain to build `cuengine`.
- The Terraform CLI version pinned in `providers.cue` when a provider schema must be generated.
- Network access to the Terraform Registry and CUE Registry.

`cuengine` evaluates the provider manifest and validates generated CUE packages. The Rust generator renders the CUE files and handles schema conversion. Release discovery reads version metadata from the Terraform Registry. When a new release is found, Terraform installs that exact provider binary and requests its schema; a release with no new versions does not install Terraform or providers. The workflow installs CUE CLI v0.16.0 only for `cue mod publish` after generation.

## Generate definitions

Generate every release listed in `providers.cue`:

```sh
cargo run --locked -- generate
```

Generate one release:

```sh
cargo run --locked -- generate --provider terraform/cloudflare --provider-version 5.22.0
```

Check for new upstream releases without installing providers or writing files:

```sh
cargo run --locked -- update --dry-run
```

Generate newly released versions and record them in the local state file:

```sh
cargo run --locked -- update
```

Generated modules are written under `generated/<provider path>/<provider release>/`. Raw schemas, Terraform lockfiles, and provenance metadata are written under `schema-snapshots/<provider path>/<provider release>/`. Both directories are ignored by Git so schema definitions are never committed to this repository.

## Scheduled workflow

`.github/workflows/generate-provider-cue.yml` checks the public Terraform Registry every six hours and can also be run manually. Automatic discovery is limited to the last 30 days, so an upstream release older than that will not be backfilled if the workflow misses it; add it explicitly to `providers.cue` when needed. On every run it uses GitHub OIDC to authenticate with the CUE Registry, checks registry tags, and uses a small GitHub Actions cache to avoid regenerating completed releases. For each generated provider release, the workflow publishes the CUE module as `v0.0.1`; the Terraform version is part of that module's path. Generated module files use `source: kind: "self"`, allowing publication directly from the ignored output directory without committing generated files. The workflow also attaches the generated files and cursor to the run as a downloadable artifact, retained for 90 days.

The workflow's trusted publisher must grant its GitHub OIDC identity permission to read and publish under the configured module namespace. Registry errors fail the run rather than being treated as an empty tag list.

Generated definitions cover provider configuration, resources, data sources, ephemeral resources, and provider functions when present. Required fields and nested block count bounds become CUE constraints. Terraform sets are represented as CUE lists, so uniqueness is not enforced. Provider-side validators and cross-field rules absent from Terraform's exported schema cannot be represented. Unsupported schema variants fail generation rather than being silently widened.
