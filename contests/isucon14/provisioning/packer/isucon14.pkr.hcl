packer {
  required_plugins {
    ansible = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/ansible"
    }
    amazon = {
      source  = "github.com/hashicorp/amazon"
      version = "~> 1"
    }
  }
}

variable "commit_hash" {
  type    = string
  default = "example-hash"
}

variable "revision" {
  type    = string
  default = "unknown"
}

locals {
  name = "isucon14_contestant-${var.commit_hash}-${formatdate("YYYYMMDD-hhmm", timestamp())}"
  ami_tags = {
    Project  = "isucon14"
    Family   = "isucon14"
    Name     = "${local.name}"
    Revision = "${var.revision}"
    Packer   = "1"
  }
  run_tags = {
    Project = "isucon14"
    Name    = "packer-${local.name}"
    Packer  = "1"
    Ignore  = "1"
  }
}

data "amazon-ami" "base-image" {
  filters = {
    name          = "isucon14_baseimage-*"
    "tag:Project" = "isucon14"
    "tag:Family"  = "isucon14"
  }
  most_recent = true
  owners      = ["self"]
  region      = "ap-northeast-1"
}

source "amazon-ebs" "isucon14" {
  ami_name    = "${local.name}"
  ami_regions = ["ap-northeast-1"]

  tags          = local.ami_tags
  snapshot_tags = local.ami_tags

  source_ami    = "${data.amazon-ami.base-image.id}"
  region        = "ap-northeast-1"
  instance_type = "c6i.4xlarge"

  run_tags        = local.run_tags
  run_volume_tags = local.run_tags

  ssh_interface           = "public_ip"
  ssh_username            = "ubuntu"
  temporary_key_pair_type = "ed25519"

  launch_block_device_mappings {
    volume_size = 16
    device_name = "/dev/sda1"
  }
}

build {
  sources = ["source.amazon-ebs.isucon14"]

  provisioner "ansible" {
    playbook_file = "../../provisioning/ansible/application.yml"
    host_alias = "application"
    use_proxy = false
  }
  provisioner "shell" {
    env = {
      DEBIAN_FRONTEND = "noninteractive"
    }
    inline = [
      # Remove authorized_keys for packer
      "sudo truncate -s 0 /home/ubuntu/.ssh/authorized_keys",
      "sudo truncate -s 0 /etc/machine-id",
      "sudo rm -f /opt/aws-env-isucon-subdomain-address.sh.lock",
    ]
  }
}
