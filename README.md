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

Generate newly released versions and record them in local state:

```sh
cargo run --locked -- update
```

Generated modules are written under `generated/<provider path>/<provider release>/`. Raw schemas, Terraform lockfiles, and provenance metadata are written under `schema-snapshots/<provider path>/<provider release>/`. Both directories are ignored by Git and are never committed.

## Automation

`.github/workflows/generate-provider-cue.yml` runs on pushes to `main`, every six hours, and on manual dispatch. Each run checks Terraform releases from the last 30 days and reads CUE Registry tags for every candidate module. It skips a release only when the matching `v<provider-version>` tag is already published. The Actions cache stores Rust build dependencies, `target`, and pending provider/version IDs; it never stores generated modules or schema snapshots.

The workflow uses GitHub OIDC trusted publishing for the CUE Registry. Rust and `cuengine` generate and validate each missing module; the CUE CLI is used only for `cue mod publish`. Generated modules and schema snapshots live under the runner's temporary directory and are discarded when the job ends. Cached pending IDs keep failed releases eligible after the 30-day discovery window. The workflow checks the CUE Registry on every run and does not commit or upload generated files.

## Schema coverage

Generated definitions cover provider configuration, resources, data sources, ephemeral resources, list resources, actions, resource identities, state stores, and provider functions when present. Required fields and nested block count bounds become CUE constraints. Terraform sets are represented as CUE lists, so uniqueness is not enforced. Provider-side validators and cross-field rules absent from Terraform's exported schema cannot be represented. Unsupported schema variants fail generation rather than being silently widened.
