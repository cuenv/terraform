// Copyright 2026 David Flanagan
// Top 100 listed Terraform Registry providers by cumulative downloads, plus explicitly requested providers.
// Baselines are stable linux_amd64 releases available on 2026-09-27.
package providers

module_prefix:         "github.com/cuenv/terraform"
cue_language_version:  "v0.16.0"
terraform_cli_version: "1.16.4"

providers: {
	// #001 downloads=7706542915 tier=official
	"terraform/hashicorp/aws": {
		source: "hashicorp/aws"
		minimumVersion: "6.66.0"
	}
	// #002 downloads=3249616808 tier=official
	"terraform/hashicorp/random": {
		source: "hashicorp/random"
		minimumVersion: "3.9.1"
	}
	// #003 downloads=2516833677 tier=official
	"terraform/hashicorp/google": {
		source: "hashicorp/google"
		minimumVersion: "8.4.0"
	}
	// #004 downloads=2505017308 tier=official
	"terraform/hashicorp/null": {
		source: "hashicorp/null"
		minimumVersion: "3.3.2"
	}
	// #005 downloads=1970091370 tier=official
	"terraform/hashicorp/azurerm": {
		source: "hashicorp/azurerm"
		minimumVersion: "5.7.0"
	}
	// #006 downloads=1659302013 tier=official
	"terraform/hashicorp/kubernetes": {
		source: "hashicorp/kubernetes"
		minimumVersion: "3.2.1"
	}
	// #007 downloads=1283477954 tier=official
	"terraform/hashicorp/google-beta": {
		source: "hashicorp/google-beta"
		minimumVersion: "8.4.0"
	}
	// #008 downloads=1272993790 tier=official
	"terraform/hashicorp/time": {
		source: "hashicorp/time"
		minimumVersion: "0.14.2"
	}
	// #009 downloads=1240930045 tier=official
	"terraform/hashicorp/local": {
		source: "hashicorp/local"
		minimumVersion: "2.9.1"
	}
	// #010 downloads=1161247245 tier=official
	"terraform/hashicorp/template": {
		source: "hashicorp/template"
		minimumVersion: "2.2.0"
	}
	// #011 downloads=1080548746 tier=official
	"terraform/hashicorp/external": {
		source: "hashicorp/external"
		minimumVersion: "2.4.2"
	}
	// #012 downloads=1009515372 tier=official
	"terraform/hashicorp/tls": {
		source: "hashicorp/tls"
		minimumVersion: "4.4.1"
	}
	// #013 downloads=925869752 tier=official
	"terraform/hashicorp/vault": {
		source: "hashicorp/vault"
		minimumVersion: "5.12.0"
	}
	// #014 downloads=847329076 tier=official
	"terraform/hashicorp/archive": {
		source: "hashicorp/archive"
		minimumVersion: "2.8.1"
	}
	// #015 downloads=777441007 tier=official
	"terraform/hashicorp/azuread": {
		source: "hashicorp/azuread"
		minimumVersion: "3.10.0"
	}
	// #016 downloads=757176162 tier=official
	"terraform/hashicorp/helm": {
		source: "hashicorp/helm"
		minimumVersion: "3.3.0"
	}
	// #017 downloads=555209487 tier=partner
	"terraform/datadog/datadog": {
		source: "datadog/datadog"
		minimumVersion: "4.22.0"
	}
	// #018 downloads=539985039 tier=partner
	"terraform/confluentinc/confluent": {
		source: "confluentinc/confluent"
		minimumVersion: "2.87.0"
	}
	// #019 downloads=411397703 tier=official
	"terraform/hashicorp/http": {
		source: "hashicorp/http"
		minimumVersion: "3.6.2"
	}
	// #020 downloads=376896463 tier=partner
	"terraform/integrations/github": {
		source: "integrations/github"
		minimumVersion: "6.13.0"
	}
	// #021 downloads=339619953 tier=partner
	"terraform/cloudflare/cloudflare": {
		source: "cloudflare/cloudflare"
		minimumVersion: "5.26.0"
	}
	// #022 downloads=285653286 tier=partner
	"terraform/azure/azapi": {
		source: "azure/azapi"
		minimumVersion: "2.12.0"
	}
	// #023 downloads=284812307 tier=official
	"terraform/hashicorp/cloudinit": {
		source: "hashicorp/cloudinit"
		minimumVersion: "2.4.1"
	}
	// #024 downloads=257187262 tier=community
	"terraform/cyrilgdn/postgresql": {
		source: "cyrilgdn/postgresql"
		minimumVersion: "1.27.0"
	}
	// #025 downloads=238879060 tier=community
	"terraform/gavinbunney/kubectl": {
		source: "gavinbunney/kubectl"
		minimumVersion: "1.19.0"
	}
	// #026 downloads=217296985 tier=partner
	"terraform/grafana/grafana": {
		source: "grafana/grafana"
		minimumVersion: "4.46.0"
	}
	// #027 downloads=216288198 tier=partner
	"terraform/databricks/databricks": {
		source: "databricks/databricks"
		minimumVersion: "1.134.0"
	}
	// #028 downloads=198262416 tier=official
	"terraform/hashicorp/tfe": {
		source: "hashicorp/tfe"
		minimumVersion: "0.81.0"
	}
	// #029 downloads=186844240 tier=partner
	"terraform/gitlabhq/gitlab": {
		source: "gitlabhq/gitlab"
		minimumVersion: "19.4.0"
	}
	// #030 downloads=153814341 tier=partner
	"terraform/pagerduty/pagerduty": {
		source: "pagerduty/pagerduty"
		minimumVersion: "3.36.0"
	}
	// #031 downloads=151612821 tier=partner
	"terraform/newrelic/newrelic": {
		source: "newrelic/newrelic"
		minimumVersion: "3.99.3"
	}
	// #032 downloads=129552909 tier=partner
	"terraform/oracle/oci": {
		source: "oracle/oci"
		minimumVersion: "9.3.0"
	}
	// #033 downloads=126001663 tier=community
	"terraform/terraform-provider-openstack/openstack": {
		source: "terraform-provider-openstack/openstack"
		minimumVersion: "3.4.0"
	}
	// #034 downloads=111442885 tier=official
	"terraform/hashicorp/consul": {
		source: "hashicorp/consul"
		minimumVersion: "2.23.0"
	}
	// #035 downloads=103017860 tier=partner
	"terraform/okta/okta": {
		source: "okta/okta"
		minimumVersion: "7.0.0"
	}
	// #036 downloads=91227942 tier=community
	"terraform/mrparkers/keycloak": {
		source: "mrparkers/keycloak"
		minimumVersion: "4.4.0"
	}
	// #037 downloads=91213258 tier=partner-premier
	"terraform/mongodb/mongodbatlas": {
		source: "mongodb/mongodbatlas"
		minimumVersion: "2.18.0"
	}
	// #038 downloads=89527890 tier=partner
	"terraform/microsoft/azuredevops": {
		source: "microsoft/azuredevops"
		minimumVersion: "1.16.0"
	}
	// #039 downloads=82647449 tier=partner
	"terraform/jfrog/artifactory": {
		source: "jfrog/artifactory"
		minimumVersion: "12.11.14"
	}
	// #040 downloads=81541197 tier=community
	"terraform/keycloak/keycloak": {
		source: "keycloak/keycloak"
		minimumVersion: "5.9.0"
	}
	// #041 downloads=76447372 tier=community
	"terraform/alekc/kubectl": {
		source: "alekc/kubectl"
		minimumVersion: "2.4.1"
	}
	// #042 downloads=75406144 tier=partner
	"terraform/snowflakedb/snowflake": {
		source: "snowflakedb/snowflake"
		minimumVersion: "2.21.0"
	}
	// #043 downloads=71679938 tier=partner
	"terraform/vmware/vsphere": {
		source: "vmware/vsphere"
		minimumVersion: "2.17.1"
	}
	// #044 downloads=70797945 tier=community
	"terraform/carlpett/sops": {
		source: "carlpett/sops"
		minimumVersion: "1.4.1"
	}
	// #045 downloads=70743938 tier=official
	"terraform/hashicorp/awscc": {
		source: "hashicorp/awscc"
		minimumVersion: "1.103.0"
	}
	// #046 downloads=67447905 tier=partner
	"terraform/aliyun/alicloud": {
		source: "aliyun/alicloud"
		minimumVersion: "1.293.0"
	}
	// #047 downloads=66075804 tier=partner
	"terraform/elastic/elasticstack": {
		source: "elastic/elasticstack"
		minimumVersion: "0.16.5"
	}
	// #048 downloads=62293088 tier=community
	"terraform/cloudposse/utils": {
		source: "cloudposse/utils"
		minimumVersion: "2.7.0"
	}
	// #049 downloads=61721598 tier=partner
	"terraform/coder/coder": {
		source: "coder/coder"
		minimumVersion: "2.18.0"
	}
	// #050 downloads=57323348 tier=community
	"terraform/kreuzwerker/docker": {
		source: "kreuzwerker/docker"
		minimumVersion: "4.6.0"
	}
	// #051 downloads=56906566 tier=community
	"terraform/dmacvicar/libvirt": {
		source: "dmacvicar/libvirt"
		minimumVersion: "0.9.9"
	}
	// #052 downloads=56061292 tier=community
	"terraform/glesys/glesys": {
		source: "glesys/glesys"
		minimumVersion: "0.18.0"
	}
	// #053 downloads=55141262 tier=partner
	"terraform/auth0/auth0": {
		source: "auth0/auth0"
		minimumVersion: "1.58.0"
	}
	// #054 downloads=52753736 tier=partner
	"terraform/rancher/rancher2": {
		source: "rancher/rancher2"
		minimumVersion: "15.1.2"
	}
	// #055 downloads=50795824 tier=official
	"terraform/hashicorp/nomad": {
		source: "hashicorp/nomad"
		minimumVersion: "2.6.1"
	}
	// #056 downloads=47898771 tier=community
	"terraform/juju/juju": {
		source: "juju/juju"
		minimumVersion: "2.3.1"
	}
	// #057 downloads=46313048 tier=community
	"terraform/fluxcd/flux": {
		source: "fluxcd/flux"
		minimumVersion: "1.9.5"
	}
	// #058 downloads=46252484 tier=partner-premier
	"terraform/1password/onepassword": {
		source: "1password/onepassword"
		minimumVersion: "3.3.1"
	}
	// #059 downloads=45956292 tier=community
	"terraform/mongey/kafka": {
		source: "mongey/kafka"
		minimumVersion: "0.13.1"
	}
	// #060 downloads=45697844 tier=partner
	"terraform/tencentcloudstack/tencentcloud": {
		source: "tencentcloudstack/tencentcloud"
		minimumVersion: "1.83.33"
	}
	// #061 downloads=44985897 tier=community
	"terraform/vancluever/acme": {
		source: "vancluever/acme"
		minimumVersion: "3.2.0"
	}
	// #062 downloads=44181025 tier=community
	"terraform/ferlab-ste-justine/etcd": {
		source: "ferlab-ste-justine/etcd"
		minimumVersion: "0.11.0"
	}
	// #063 downloads=41224660 tier=community
	"terraform/petoju/mysql": {
		source: "petoju/mysql"
		minimumVersion: "3.0.100"
	}
	// #064 downloads=39280600 tier=partner
	"terraform/sumologic/sumologic": {
		source: "sumologic/sumologic"
		minimumVersion: "3.3.2"
	}
	// #065 downloads=38336845 tier=community
	"terraform/terraform-aws-modules/http": {
		source: "terraform-aws-modules/http"
		minimumVersion: "2.4.1"
	}
	// #066 downloads=35989874 tier=community
	"terraform/opsgenie/opsgenie": {
		source: "opsgenie/opsgenie"
		minimumVersion: "0.6.40"
	}
	// #067 downloads=35461547 tier=community
	"terraform/harness/harness": {
		source: "harness/harness"
		minimumVersion: "0.45.8"
	}
	// #068 downloads=34851151 tier=community
	"terraform/mastercard/restapi": {
		source: "mastercard/restapi"
		minimumVersion: "3.0.0"
	}
	// #069 downloads=34267863 tier=community
	"terraform/ferlab-ste-justine/netaddr": {
		source: "ferlab-ste-justine/netaddr"
		minimumVersion: "0.6.0"
	}
	// #070 downloads=33768782 tier=partner
	"terraform/ovh/ovh": {
		source: "ovh/ovh"
		minimumVersion: "2.21.0"
	}
	// #071 downloads=32809382 tier=community
	"terraform/opensearch-project/opensearch": {
		source: "opensearch-project/opensearch"
		minimumVersion: "2.6.0"
	}
	// #072 downloads=31688079 tier=official
	"terraform/ibm-cloud/ibm": {
		source: "ibm-cloud/ibm"
		minimumVersion: "2.6.2"
	}
	// #073 downloads=31533578 tier=community
	"terraform/yandex-cloud/yandex": {
		source: "yandex-cloud/yandex"
		minimumVersion: "0.230.0"
	}
	// #074 downloads=30945512 tier=community
	"terraform/l-with/ldap": {
		source: "l-with/ldap"
		minimumVersion: "0.15.0"
	}
	// #075 downloads=29612654 tier=partner
	"terraform/aztfmod/azurecaf": {
		source: "aztfmod/azurecaf"
		minimumVersion: "1.2.34"
	}
	// #076 downloads=28557032 tier=partner
	"terraform/infobloxopen/infoblox": {
		source: "infobloxopen/infoblox"
		minimumVersion: "2.13.0"
	}
	// #077 downloads=27770528 tier=community
	"terraform/cyrilgdn/rabbitmq": {
		source: "cyrilgdn/rabbitmq"
		minimumVersion: "1.10.1"
	}
	// #078 downloads=25987314 tier=partner
	"terraform/ns1-terraform/ns1": {
		source: "ns1-terraform/ns1"
		minimumVersion: "2.9.0"
	}
	// #079 downloads=25922518 tier=community
	"terraform/jianyuan/sentry": {
		source: "jianyuan/sentry"
		minimumVersion: "0.15.7"
	}
	// #080 downloads=25804497 tier=partner
	"terraform/elastic/ec": {
		source: "elastic/ec"
		minimumVersion: "0.13.1"
	}
	// #081 downloads=25312440 tier=partner
	"terraform/spotinst/spotinst": {
		source: "spotinst/spotinst"
		minimumVersion: "1.241.1"
	}
	// #082 downloads=25140754 tier=community
	"terraform/goharbor/harbor": {
		source: "goharbor/harbor"
		minimumVersion: "3.12.5"
	}
	// #083 downloads=24840126 tier=partner
	"terraform/groundcover-com/groundcover": {
		source: "groundcover-com/groundcover"
		minimumVersion: "1.22.5"
	}
	// #084 downloads=24272829 tier=partner
	"terraform/launchdarkly/launchdarkly": {
		source: "launchdarkly/launchdarkly"
		minimumVersion: "3.1.6"
	}
	// #085 downloads=23924690 tier=partner
	"terraform/dynatrace-oss/dynatrace": {
		source: "dynatrace-oss/dynatrace"
		minimumVersion: "1.105.0"
	}
	// #086 downloads=23862296 tier=partner
	"terraform/azure/modtm": {
		source: "azure/modtm"
		minimumVersion: "0.4.0"
	}
	// #087 downloads=22676791 tier=partner
	"terraform/fastly/fastly": {
		source: "fastly/fastly"
		minimumVersion: "9.7.0"
	}
	// #088 downloads=21394008 tier=community
	"terraform/phillbaker/elasticsearch": {
		source: "phillbaker/elasticsearch"
		minimumVersion: "2.0.7"
	}
	// #089 downloads=21327880 tier=community
	"terraform/argoproj-labs/argocd": {
		source: "argoproj-labs/argocd"
		minimumVersion: "7.17.0"
	}
	// #090 downloads=21320980 tier=community
	"terraform/scottwinkler/shell": {
		source: "scottwinkler/shell"
		minimumVersion: "1.7.10"
	}
	// #091 downloads=21254529 tier=community
	"terraform/elastic-infra/ldap": {
		source: "elastic-infra/ldap"
		minimumVersion: "2.0.1"
	}
	// #092 downloads=20676117 tier=partner
	"terraform/rollbar/rollbar": {
		source: "rollbar/rollbar"
		minimumVersion: "1.18.0"
	}
	// #093 downloads=20155284 tier=community
	"terraform/cloudposse/awsutils": {
		source: "cloudposse/awsutils"
		minimumVersion: "0.20.1"
	}
	// #094 downloads=19977293 tier=community
	"terraform/aminueza/minio": {
		source: "aminueza/minio"
		minimumVersion: "3.43.0"
	}
	// #095 downloads=19952379 tier=official
	"terraform/hashicorp/hcp": {
		source: "hashicorp/hcp"
		minimumVersion: "0.114.0"
	}
	// #096 downloads=19729564 tier=community
	"terraform/hashicorp/googleworkspace": {
		source: "hashicorp/googleworkspace"
		minimumVersion: "0.7.0"
	}
	// #097 downloads=19526033 tier=community
	"terraform/goauthentik/authentik": {
		source: "goauthentik/authentik"
		minimumVersion: "2026.8.0"
	}
	// #098 downloads=19365167 tier=community
	"terraform/telmate/proxmox": {
		source: "telmate/proxmox"
		minimumVersion: "2.9.14"
	}
	// #099 downloads=19142049 tier=partner
	"terraform/hetznercloud/hcloud": {
		source: "hetznercloud/hcloud"
		minimumVersion: "1.69.0"
	}
	// #100 downloads=19088905 tier=partner
	"terraform/buildkite/buildkite": {
		source: "buildkite/buildkite"
		minimumVersion: "1.39.2"
	}
	// Explicitly requested on 2026-09-27; downloads=2716063 tier=community
	"terraform/infisical/infisical": {
		source: "infisical/infisical"
		minimumVersion: "0.19.36"
	}
}
