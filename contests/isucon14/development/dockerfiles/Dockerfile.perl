FROM perl:5.40.0-bookworm

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install --no-install-recommends -y \
    curl wget libssl-dev ca-certificates lsb-release openssl \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*


RUN set -eux; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
		bzip2 \
		openssl \
# FATAL ERROR: please install the following Perl modules before executing /usr/local/mysql/scripts/mysql_install_db:
# File::Basename
# File::Copy
# Sys::Hostname
# Data::Dumper
		perl \
		xz-utils \
		zstd \
	; \
	rm -rf /var/lib/apt/lists/*

RUN set -eux; \
# pub   rsa4096 2023-10-23 [SC] [expires: 2025-10-22]
#       BCA4 3417 C3B4 85DD 128E  C6D4 B7B3 B788 A8D3 785C
# uid           [ unknown] MySQL Release Engineering <mysql-build@oss.oracle.com>
# sub   rsa4096 2023-10-23 [E] [expires: 2025-10-22]
	key='BCA4 3417 C3B4 85DD 128E C6D4 B7B3 B788 A8D3 785C'; \
	export GNUPGHOME="$(mktemp -d)"; \
	gpg --batch --keyserver keyserver.ubuntu.com --recv-keys "$key"; \
	mkdir -p /etc/apt/keyrings; \
	gpg --batch --export "$key" > /etc/apt/keyrings/mysql.gpg; \
	gpgconf --kill all; \
	rm -rf "$GNUPGHOME"

ENV MYSQL_MAJOR 8.0
ENV MYSQL_VERSION 8.0.40-1debian12

RUN echo 'deb [ signed-by=/etc/apt/keyrings/mysql.gpg ] http://repo.mysql.com/apt/debian/ bookworm mysql-8.0' > /etc/apt/sources.list.d/mysql.list

# the "/var/lib/mysql" stuff here is because the mysql-server postinst doesn't have an explicit way to disable the mysql_install_db codepath besides having a database already "configured" (ie, stuff in /var/lib/mysql/mysql)
# also, we set debconf keys to make APT a little quieter
RUN { \
		echo mysql-community-server mysql-community-server/data-dir select ''; \
		echo mysql-community-server mysql-community-server/root-pass password ''; \
		echo mysql-community-server mysql-community-server/re-root-pass password ''; \
		echo mysql-community-server mysql-community-server/remove-test-db select false; \
	} | debconf-set-selections \
	&& apt-get update \
	&& apt-get install -y \
		mysql-community-client="${MYSQL_VERSION}" \
		libmysqlclient-dev="${MYSQL_VERSION}" \
		libmysqlclient21="${MYSQL_VERSION}"

RUN useradd --uid=1001 --create-home isucon
USER isucon

WORKDIR /home/isucon/webapp/perl

COPY cpanfile ./
RUN cpm install --show-build-log-on-failure --without-test

COPY --chown=isucon:isucon ./lib /home/isucon/webapp/perl/lib
COPY --chown=isucon:isucon ./cpanfile ./app.psgi ./app.pl /home/isucon/webapp/perl/
ENV PERL5LIB=/home/isucon/webapp/perl/local/lib/perl5
ENV PATH=/home/isucon/webapp/perl/local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

EXPOSE 8080
CMD ["./local/bin/plackup", "-s", "Starlet", "-p", "8080", "-Ilib", "-r", "app.psgi"]

