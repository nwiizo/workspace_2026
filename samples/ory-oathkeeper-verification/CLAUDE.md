# Ory Oathkeeper Verification

## Overview

Oathkeeper + Keto integration verification environment.
Tests Zero Trust API Gateway pattern with fine-grained authorization.

## Quick Start

```sh
docker compose up -d
bash scripts/e2e-test.sh
```

## Architecture

```
Client -> Oathkeeper (4455) -> Backend (nginx)
              |
              +-> Keto (4466) - Authorization checks
```

## Access Rules

| Endpoint | Auth | Authorization |
|----------|------|---------------|
| `/health` | anonymous | allow |
| `/api/public` | anonymous | allow |
| `/api/protected` | noop | allow (header mutator) |
| `/api/documents/{id}` GET | noop | Keto viewer check |
| `/api/documents/{id}` PUT/DELETE | noop | Keto editor check |

## Test Users

- **alice**: editor + viewer of doc1
- **bob**: viewer of doc1
- **charlie**: no permissions

## Commands

```sh
# Start services
docker compose up -d

# Run tests
bash scripts/e2e-test.sh

# View logs
docker compose logs oathkeeper
docker compose logs keto

# Stop services
docker compose down
```
