mod cue;
mod generate;
mod manifest;
mod registry;
mod schema;
mod terraform;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "terraform-cue",
    about = "Fetch Terraform provider schemas and generate versioned CUE definition files"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fetch pinned provider schemas and generate versioned CUE definition files.
    Generate {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long = "provider-version")]
        provider_version: Option<String>,
        #[arg(long, default_value = "generated")]
        output: PathBuf,
        #[arg(long, default_value = "schema-snapshots")]
        snapshots: PathBuf,
    },
    /// Generate newly released definitions and optionally publish missing CUE Registry tags.
    Update {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long, default_value = "generated")]
        output: PathBuf,
        #[arg(long, default_value = "schema-snapshots")]
        snapshots: PathBuf,
        #[arg(long, default_value = ".provider-release-state.json")]
        state_file: PathBuf,
        /// List missing releases without generating CUE files; update pending release IDs.
        #[arg(long)]
        dry_run: bool,
        /// Publish generated provider modules to the CUE Registry after validation.
        #[arg(long)]
        publish: bool,
    },
    /// Print the Terraform CLI version pinned in providers.cue.
    TerraformVersion,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    match cli.command {
        Command::Generate {
            provider,
            provider_version,
            output,
            snapshots,
        } => generate::run(
            &root,
            provider.as_deref(),
            provider_version.as_deref(),
            &output,
            &snapshots,
        ),
        Command::Update {
            provider,
            output,
            snapshots,
            state_file,
            dry_run,
            publish,
        } => generate::update(
            &root,
            provider.as_deref(),
            &output,
            &snapshots,
            &state_file,
            dry_run,
            publish,
        ),
        Command::TerraformVersion => {
            let manifest = manifest::load(&root)?;
            println!("{}", manifest.terraform_cli_version);
            Ok(())
        }
    }
}
