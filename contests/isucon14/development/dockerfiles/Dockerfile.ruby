FROM ruby:3.3.6-bookworm

WORKDIR /home/isucon/webapp/ruby

RUN apt-get update && apt-get install --no-install-recommends -y \
  default-mysql-client-core=1.1.0 \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

COPY Gemfile Gemfile.lock ./
ENV BUNDLE_DEPLOYMENT=1 BUNDLE_PATH=/gems BUNDLE_JOBS=8
RUN bundle install

COPY . .

ENV RUBY_YJIT_ENABLE=1

EXPOSE 8080
CMD ["bundle", "exec", "puma", "--bind", "tcp://0.0.0.0:8080", "--workers", "8", "--threads", "0:8", "--environment", "production"]
