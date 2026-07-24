ARG SV_IMAGE="703671906592.dkr.ecr.ap-northeast-1.amazonaws.com/prod/benchmarker:supervisor"
FROM ${SV_IMAGE} AS supervisor

FROM public.ecr.aws/docker/library/golang:bookworm AS builder

WORKDIR /app
COPY . .
RUN go build -o /app/bench -ldflags "-s -w" -trimpath .

FROM public.ecr.aws/docker/library/debian:bookworm-slim

RUN apt-get update
RUN apt install -y ca-certificates openssl curl
RUN bash -c "curl -sSfL https://raw.githubusercontent.com/aquaproj/aqua-installer/v3.1.0/aqua-installer | bash"
ENV PATH=/root/.local/share/aquaproj-aqua/bin:$PATH
ENV AQUA_GLOBAL_CONFIG=/etc/aqua/aqua.yaml
COPY aqua.yaml /etc/aqua/
RUN aqua i -a
WORKDIR /app
COPY --from=builder /app/bench /app/bench
COPY --from=supervisor /usr/local/bin/isuxportal-supervisor /app/supervisor
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/app/supervisor", "/app/bench", "run"]
