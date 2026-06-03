# DONADONA OAuth BFF Sample

## BFF Environment

The Rust backend exposes `/api/bff/*` so SPA code never receives OAuth access or refresh tokens.

| Variable | Purpose | Local default |
| --- | --- | --- |
| `HYDRA_PUBLIC_URL` | Hydra public token and authorization endpoint base URL | `http://localhost:4444` |
| `BFF_AUTHORIZATION_URL` | Browser-visible Hydra authorization endpoint base URL | `http://localhost:4444` |
| `BFF_CLIENT_ID` | Confidential OAuth client ID used by the BFF | `demo-client` |
| `BFF_CLIENT_SECRET` | Confidential OAuth client secret used by the BFF | `demo-secret` |
| `BFF_REDIRECT_URI` | Redirect URI handled by Rust | `http://localhost:3000/api/bff/callback` |
| `BFF_FRONTEND_ORIGIN` | Exact SPA origin allowed by CORS and Origin checks | `http://localhost:3002` |
| `BFF_API_UPSTREAM_URL` | Internal API origin that `/api/bff/proxy/*` can call | `http://auth-provider:3000` |
| `NEXT_PUBLIC_BFF_BASE_URL` | Browser-visible BFF base URL used by the SPA | `http://localhost:3000` |

Run `./scripts/create-client.sh` after Hydra is ready to register the demo confidential client.
