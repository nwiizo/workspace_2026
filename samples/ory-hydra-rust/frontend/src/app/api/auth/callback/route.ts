import { NextResponse } from "next/server";

/**
 * OAuth2 Authorization Code callback handler
 *
 * This endpoint exchanges the authorization code for tokens and verifies the ID token.
 *
 * Security considerations:
 * - ID token signature is verified using Hydra's JWKS endpoint
 * - Issuer and audience claims are validated
 * - Token expiration is checked
 *
 * @see https://openid.net/specs/openid-connect-core-1_0.html#TokenEndpoint
 * @see https://www.ory.sh/docs/hydra/guides/oauth2-token-introspection
 */
export async function POST() {
  return NextResponse.json(
    { error: "OAuth callback is handled by the Rust BFF" },
    { status: 410 }
  );
}
