# Terraform provider CUE definitions

A Rust generator for CUE definitions derived from Terraform provider schemas. The explicit provider set lives in `providers.cue` and contains the 100 listed Terraform Registry providers with the most cumulative downloads, ranked from a Registry metadata snapshot taken on 2026-09-27, plus any providers added explicitly. Each entry has a `minimumVersion` set to a stable release that provides a `linux_amd64` package. The updater generates that baseline and checks newer stable releases published in the last 30 days.

Provider paths follow the Terraform Registry address: `terraform/<namespace>/<type>`. For example, `terraform/hashicorp/aws` refers to `hashicorp/aws`. To add or remove providers, edit `providers.cue`.

## Schema generation

The Registry API provides release metadata and provider package download locations, but not a complete machine-readable provider schema. Schema generation uses the pinned Terraform CLI to install the exact provider release and call `terraform providers schema -json`. Rust handles release discovery, schema conversion, file output, and integrity metadata. `cuengine` evaluates and validates the generated CUE; the CUE CLI is not used for generation or validation.

Requirements are Rust stable, Go 1.26 or newer, a C toolchain for `cuengine`, and Terraform CLI 1.16.4 for schema generation.

Generate every pinned provider release:

```sh
cargo run --locked -- generate
```

Generate one release:

```sh
cargo run --locked -- generate --provider terraform/hashicorp/aws --provider-version 6.66.0
```

Check for newer stable releases from the explicit provider list without installing providers or generating CUE files:

```sh
cargo run --locked -- update --dry-run
```

Generated modules are written under `generated/<provider path>/<CUE module version without the leading v>/`. Raw schemas, Terraform lockfiles, and provenance metadata are written under `schema-snapshots/<provider path>/<CUE module version without the leading v>/`. Both directories are ignored by Git and are never committed.

## GHCR registry and publishing

CUE modules use Terraform-compatible logical paths such as `ghcr.io/cuenv/terraform/hashicorp/aws@v6`. The CUE release version is the exact Terraform provider version: Terraform `6.66.0` becomes CUE module version `v6.66.0`. `cue-registry.cue` maps those module paths into the single OCI package `ghcr.io/cuenv/terraform-cue`; CUE's `hashAsTag` encoding keeps the logical provider module paths distinct inside that package.

Provider versions are immutable publication identities. The publisher skips an existing GHCR tag and records it as published, so a generator change never replaces definitions for an existing Terraform release. Each workflow run lists GHCR tags and checks configured baselines, cached published releases, pending releases, and upstream releases from the last 30 days. The Actions cache is the release index; if GitHub evicts it, an older unpublished release outside that discovery window may need to be added back to `providers.cue`. Generated schemas and snapshots stay temporary and are never committed.

Generate releases without credentials and record them as pending:

```sh
cargo run --locked -- update --defer-publish \
  --output /tmp/provider-cue/generated \
  --snapshots /tmp/provider-cue/snapshots \
  --state-file /tmp/provider-cue/provider-release-state.json
```

The Actions workflow checks existing GHCR tags in a short-lived read-only step before generation. It runs schema generation in a job without publish credentials, then passes temporary generated modules to a separate GHCR publisher job. That job checks tags again, verifies that CUE resolves each logical module to the expected GHCR package and tag, and invokes `cue mod publish` only for missing tags. It does not run Terraform or provider binaries.

The publisher runs only in the serialized `main` GitHub Actions workflow. This is the sole supported writer: it checks for an existing tag, checks again immediately before a push, and skips tags it finds. GHCR publication through CUE does not offer an atomic create-only operation, so keep package write access limited to this workflow. You can inspect current package tags locally with `GHCR_USERNAME=... GHCR_TOKEN=... cargo run --locked -- registry-tags`.

## Using the modules

From a CUE project (create one with `cue mod init example.com/myproject` if needed), set `CUE_REGISTRY` to this repository's `cue-registry.cue` file and request the exact provider release:

```sh
export CUE_REGISTRY="file:/path/to/terraform/cue-registry.cue"
cue mod get ghcr.io/cuenv/terraform/hashicorp/aws@v6.66.0
```

Keep this registry setting in the shell or CI environment for later imports and dependency fetches as well.

Then import the module's major path and unify a configuration with its generated definition:

```cue
package example

import awsProvider "ghcr.io/cuenv/terraform/hashicorp/aws@v6"

provider: awsProvider.#ProviderConfig & {
	region: "eu-west-2"
}
```

GHCR makes a new package private by default. To allow end users to pull without credentials, an owner must change the package visibility to public. Private packages require GHCR read access and Docker-compatible credentials. The workflow's generation job needs package read access to check tags; only the separate publisher job receives package write permission. A manual workflow run must target `main`; other refs are blocked from publication.

## Schema coverage

Generated definitions cover provider configuration, resources, data sources, ephemeral resources, list resources, actions, resource identities, state stores, and provider functions when present. Resource types are exposed under category namespaces such as `Resource.LocalSensitiveFile`, and Terraform field names are converted recursively to lower camelCase. Named Terraform schema types and fields carry `@terraform(name="...")` attributes with their original Terraform names. A converter must read CUE declaration and field attributes; `cuengine`'s evaluated JSON output omits those attributes. Required fields and nested block count bounds become CUE constraints. Terraform sets are represented as CUE lists, so uniqueness is not enforced. Provider-side validators and cross-field rules absent from Terraform's exported schema cannot be represented. Unsupported schema variants and name collisions after camelCase conversion fail generation rather than being silently widened.
