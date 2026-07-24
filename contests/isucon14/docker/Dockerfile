FROM public.ecr.aws/docker/library/node:22.11.0-bookworm-slim AS frontend

RUN npm install --global pnpm@9.15.4

WORKDIR /workspace
COPY frontend/package.json frontend/pnpm-lock.yaml ./frontend/
RUN pnpm --dir frontend install --frozen-lockfile

COPY frontend ./frontend
RUN mkdir -p ./bench/benchrun \
    && pnpm --dir frontend run build

FROM public.ecr.aws/docker/library/nginx:1.27-bookworm AS nginx

RUN apt-get update \
    && apt-get install --no-install-recommends --yes curl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=frontend /workspace/frontend/build/client /home/isucon/webapp/public

EXPOSE 80

FROM public.ecr.aws/docker/library/golang:1.23-bookworm AS benchmark-builder

WORKDIR /src
COPY bench/go.mod bench/go.sum ./
RUN go mod download

COPY bench ./
COPY --from=frontend /workspace/bench/benchrun/frontend_files.json ./benchrun/frontend_files.json
COPY --from=frontend /workspace/bench/benchrun/frontend_hashes.json ./benchrun/frontend_hashes.json
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/bench .

FROM public.ecr.aws/docker/library/debian:bookworm-slim AS benchmark

RUN apt-get update \
    && apt-get install --no-install-recommends --yes ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=benchmark-builder /out/bench /usr/local/bin/bench

ENTRYPOINT ["/usr/local/bin/bench"]
CMD ["run"]
