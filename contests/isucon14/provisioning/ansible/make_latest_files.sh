#!/usr/bin/env bash

set -eux
cd $(dirname $0)

cd ../../frontend
make
cp -r ./build/client/ ../webapp/public/
cd ../provisioning/ansible

cd ../../bench
task build-linux-amd64
mkdir -p ../provisioning/ansible/roles/bench/files
mv bin/bench_linux_amd64 ../provisioning/ansible/roles/bench/files
cd ../provisioning/ansible

cd ../../envcheck/isucon-env-checker
task build-linux-amd64
cd ../../provisioning/ansible

cd ../../
tar -zcvf webapp.tar.gz webapp
mv webapp.tar.gz provisioning/ansible/roles/webapp/files

cd ./provisioning/ansible
