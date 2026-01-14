# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**Ory Keto Authorization Verification** - A minimal setup to verify Ory Keto permission system based on Google's Zanzibar model. This is the verification environment for blog series article 05.

## Key Concepts

### Zanzibar Model
- **Namespace**: A type of object (User, Organization, Project, Document)
- **Object**: An instance within a namespace (e.g., `Project:project-alpha`)
- **Relation**: A relationship type (owner, editor, viewer, member)
- **Subject**: Who has the relation (User, or another object via subject set)

### Relation Tuple Format
```
namespace:object#relation@subject
```

Examples:
```
Organization:acme#member@User:alice
Project:alpha#owner@User:bob
Document:doc1#viewer@Organization:acme#member
```

## Quick Start

```sh
# Start Keto
docker compose up -d

# Health check
curl http://localhost:4466/health/ready

# Create a relation tuple (Write API)
curl -X PUT http://localhost:4467/admin/relation-tuples \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "alice"
  }'

# Check permission (Read API)
curl -X POST http://localhost:4466/relation-tuples/check \
  -H "Content-Type: application/json" \
  -d '{
    "namespace": "Organization",
    "object": "acme",
    "relation": "member",
    "subject_id": "alice"
  }'
# Response: {"allowed": true}

# List relation tuples
curl "http://localhost:4466/relation-tuples?namespace=Organization"

# Stop
docker compose down -v
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Application │────▶│  Ory Keto   │────▶│ PostgreSQL  │
│             │     │ Read: 4466  │     │             │
│             │     │ Write: 4467 │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Port Mapping

| Port | Service | Description |
|------|---------|-------------|
| 4466 | Keto Read API | Permission checks, list tuples |
| 4467 | Keto Write API | Create/delete relation tuples |
| 4468 | Metrics | Prometheus metrics |

## Namespaces

| ID | Name | Description |
|----|------|-------------|
| 0 | User | System users |
| 1 | Organization | Tenant/company |
| 2 | Project | Project within organization |
| 3 | Document | Document within project |

## Permission Model Example

```
Organization:acme
├── member: [alice, bob, charlie]
└── admin: [alice]

Project:alpha (parent: Organization:acme)
├── owner: [alice]
├── editor: [bob]
└── viewer: [Organization:acme#member]  ← All acme members can view

Document:doc1 (parent: Project:alpha)
├── editor: [alice]
└── viewer: [Project:alpha#editor]  ← Inherited from project editors
```

## API Reference

### Write API (4467)

```sh
# Create relation tuple
PUT /admin/relation-tuples

# Delete relation tuple
DELETE /admin/relation-tuples?namespace=...&object=...&relation=...&subject_id=...
```

### Read API (4466)

```sh
# Check permission
POST /relation-tuples/check

# List relation tuples
GET /relation-tuples?namespace=...

# Expand (show who has access)
POST /relation-tuples/expand
```

## Related Projects

- `../ory-hydra-rust/` - OAuth2/OIDC with custom Login Provider (blog 01-03)
- `../ory-kratos-verification/` - Identity management (blog 04)
