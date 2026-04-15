# argocd-pr-preview-demo

Kind based verification sandbox for pull-request preview environments with Argo CD.

## What This Verifies

- A separate Git repository is mirrored and served inside the cluster as the GitOps source.
- Argo CD discovers `preview-metadata/*.yaml` metadata files and creates one `Application` per preview environment.
- Each preview environment includes both frontend and backend routes on the same host.
- Create, update, and delete are driven by Git changes while the preview shape itself is rendered by `ApplicationSet` and `templatePatch`.

## Repository Layout

- This directory is the control repo: scripts, Kind config, Argo CD manifests, and screenshots.
- `~/ghq/github.com/nwiizo/argocd-pr-preview-demo-mirror` is the separate mirrored Git repository created by the scripts.
- `runtime/git-mirror/argocd-pr-preview-demo-mirror.git` is the bare mirror mounted into Kind and served by an in-cluster `git daemon`.
- `screenshots/` contains Playwright captures taken against the preview hosts.

## Argo CD Focus

This environment is meant for two tracks:

1. Local deterministic verification with the `git` files generator.
2. Newer or real-world Argo CD features, especially `ApplicationSet`, `goTemplate`, `templatePatch`, and the `pullRequest` generator example in `manifests/argocd/applicationset-pull-request.example.yaml`.

For local work, the `git` files generator is the default because it is reproducible without GitHub credentials. For a real GitHub flow, switch to the pull request generator example and point it at your SCM and mirror repo.

The local `ApplicationSet` intentionally allows delete. Using `applicationsSync: create-update` looks attractive for safety tests, but it blocks preview cleanup when a PR directory disappears.

## Quick Start

```bash
./scripts/bootstrap-kind.sh
./scripts/preview-upsert.sh 101 abc1234
./scripts/smoke-test.sh 101 abc1234
```

Useful manual checks:

```bash
curl -H 'Host: pr-101.preview.localtest.me' http://127.0.0.1:8080/
curl -H 'Host: pr-101.preview.localtest.me' http://127.0.0.1:8080/api
./scripts/preview-delete.sh 101
```

## Notes

- Hostnames use `*.preview.localtest.me`, which resolves to `127.0.0.1`, so no local DNS setup is required.
- The frontend is a static Nginx page that calls `/api`; the backend is an echo server with preview-specific text.
- The local helper scripts now write only `preview-metadata/*.yaml`. The actual per-preview shape, including the namespace name, is produced by Argo CD `ApplicationSet` with `templatePatch`.
- The generated `Application` objects carry `resources-finalizer.argocd.argoproj.io` so preview deletion cascades into child resources instead of orphaning them.
- Once the preview namespace is managed as an explicit resource, `CreateNamespace=true` is no longer needed and can interfere with prune behavior.
- `bootstrap-kind.sh` shortens Argo CD repo cache settings to `15s` for local feedback loops.
- On Kind, Argo CD can leave the `Application` health as `Progressing` because the Ingress has no external address, even when frontend and backend are already reachable.
- The current scripts are for local reproduction only. The target operational design is to push preview metadata into Git or use the `pullRequest` generator directly, then let Argo CD reconcile from there.
- This is intentionally small. The goal is to validate lifecycle mechanics and Argo CD behavior before introducing real application images or a database layer.
