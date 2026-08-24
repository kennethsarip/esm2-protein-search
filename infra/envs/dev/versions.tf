terraform {
  required_version = "~> 1.15"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }

  # Values must match infra/bootstrap outputs: state_bucket_name and
  # aws_region. Terraform backend blocks cannot reference variables or another
  # module's outputs, so this is intentionally literal.
  #
  # use_lockfile, not dynamodb_table: Terraform 1.15 deprecates the DynamoDB
  # locking parameter in favour of S3 conditional writes, which lock against
  # an object beside the state file. One less resource, one less permission,
  # one less teardown step.
  backend "s3" {
    bucket       = "esm2-protein-search-tfstate"
    key          = "envs/dev/terraform.tfstate"
    region       = "us-west-2"
    use_lockfile = true
    encrypt      = true
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "esm2-protein-search"
      ManagedBy   = "terraform"
      Environment = "dev"
    }
  }
}
