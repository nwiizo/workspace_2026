terraform {
  required_providers {
    aws = { source = "hashicorp/aws", version = ">= 5.0" }
  }
}

variable "endpoint" { type = string }

provider "aws" {
  region                      = "us-east-1"
  access_key                  = "test"
  secret_key                  = "test"
  skip_credentials_validation = true
  skip_metadata_api_check     = true
  skip_requesting_account_id  = true

  endpoints {
    s3       = var.endpoint
    dynamodb = var.endpoint
    sqs      = var.endpoint
    sns      = var.endpoint
  }

  s3_use_path_style = true
}

resource "aws_s3_bucket" "demo" {
  bucket = "tf-demo-${substr(md5(var.endpoint), 0, 6)}"
}

resource "aws_s3_object" "obj" {
  bucket  = aws_s3_bucket.demo.id
  key     = "hello.txt"
  content = "hello from terraform"
}

resource "aws_dynamodb_table" "users" {
  name         = "tf-users"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "id"
  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_sqs_queue" "main" {
  name = "tf-queue"
}

output "bucket" { value = aws_s3_bucket.demo.id }
output "queue"  { value = aws_sqs_queue.main.url }
