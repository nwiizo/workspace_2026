export BRANCH := $(shell git branch --contains | fgrep '*' | cut -d ' ' -f 2)
.DEFAULT_GOAL := all

.PHONY: all
all:
	make echo-commit_hash
	make app-build
	make init
	make build

.PHONY: base-build
base-build:
	make echo-commit_hash
	packer init -upgrade isucon14_base_image.pkr.hcl
	packer build -var "commit_hash=$$(git rev-parse HEAD)" isucon14_base_image.pkr.hcl

.PHONY: echo-commit_hash
echo-commit_hash:
	git rev-parse HEAD

.PHONY: app-build
app-build: 
	cd ../../provisioning/ansible/ && ./make_latest_files.sh

.PHONY: init
init:
	packer init -upgrade isucon14.pkr.hcl

.PHONY: build
build:
	packer build -var "commit_hash=$$(git rev-parse HEAD)" isucon14.pkr.hcl
