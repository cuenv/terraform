// Copyright 2026 David Flanagan
package providers

module_prefix:         "github.com/rawkode/pulumi-cue"
module_major:          0
cue_language_version:  "v0.16.0"
terraform_cli_version: "1.16.4"

providers: {
	"terraform/cloudflare": {
		source: "cloudflare/cloudflare"
		versions: {"5.22.0": {}}
	}
}
