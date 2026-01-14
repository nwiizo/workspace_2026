# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Ory Kratos + Hydra Integration Verification** - A minimal setup to verify Ory Kratos identity management integrated with Ory Hydra OAuth2/OIDC server. This is the verification environment for blog series article 04.

**Key Architecture**:
- Kratos handles user authentication (login, registration, password reset, MFA)
- Hydra handles OAuth2/OIDC protocol (token issuance, client management)
- Kratos UI provides self-service authentication pages

## Quick Start

```sh
# Start all services
docker compose up -d

# View logs
docker compose logs -f

# Health check
curl http://localhost:4433/health/ready  # Kratos
curl http://localhost:4444/health/ready  # Hydra

# Stop all services
docker compose down -v
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Browser   │     │ Kratos UI   │     │   Kratos    │
│  (User)     │────▶│ (Port 4455) │────▶│ (Port 4433) │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼
                    ┌─────────────┐     ┌─────────────┐
                    │   Hydra     │◀────│ PostgreSQL  │
                    │ (Port 4444) │     │ (Port 5432) │
                    └─────────────┘     └─────────────┘
```

## Port Mapping

| Service | Port | Description |
|---------|------|-------------|
| Kratos Public | 4433 | Public API (login, registration, etc.) |
| Kratos Admin | 4434 | Admin API (identity management) |
| Hydra Public | 4444 | OAuth2/OIDC endpoints |
| Hydra Admin | 4445 | Client management, consent |
| Kratos UI | 4455 | Self-service UI pages |
| PostgreSQL | 5432 | Database |

## Configuration Files

- `docker-compose.yml` - Service orchestration
- `kratos/kratos.yml` - Kratos configuration
- `kratos/identity.schema.json` - Identity schema definition
- `hydra/hydra.yml` - Hydra configuration

## Testing OAuth2 Flow

```sh
# Run the test script
./scripts/test-flow.sh

# Or manually:
# 1. Register a user via Kratos UI
open http://localhost:4455/registration

# 2. Create OAuth2 client
docker compose exec hydra hydra create oauth2-client \
  --endpoint http://localhost:4445 \
  --grant-type authorization_code \
  --response-type code \
  --scope openid,profile,email \
  --redirect-uri http://localhost:8080/callback

# 3. Start authorization flow
# Open: http://localhost:4444/oauth2/auth?client_id=<CLIENT_ID>&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=test
```

## Related Project

- `../ory-hydra-rust/` - Custom Login Provider implementation (blog 01-03)
