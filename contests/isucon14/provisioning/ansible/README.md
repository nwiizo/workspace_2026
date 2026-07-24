# Ansible

## 利用方法

```
# ベンチマーカ、webappのアーカイブをアップデート　
$ ./make_latest_files.sh

# ローカルの場合
$ ansible-playbook -i inventory/localhost application.yml
$ ansible-playbook -i inventory/localhost benchmark.yml

# sacloud試し環境へのリモート実行
$ ansible-playbook -i inventory/sacloud application.yml
$ ansible-playbook -i inventory/sacloud benchmark.yml

```

すでに対象サーバに /home/isucon/webapp/sql がある場合、tarをアップロードするだけで展開はしません

## 証明書について

`dummy.crt` は下記スクリプトで生成可能

```bash
#!/bin/bash
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout server.key -out server.crt -days 365 -nodes -subj "/CN=isucon.net"
```

## make_lastest_filesの中身

```
$ cd bench
$ make linux_amd64
$ mkdir -p ../provisioning/ansible/roles/bench/files
$ mv bin/bench_linux_amd64 ../provisioning/ansible/roles/bench/files
$ cd ..
$ tar -zcvf webapp.tar.gz webapp
$ mv webapp.tar.gz provisioning/ansible/roles/webapp/files
$ cd provisioning/ansible
```
