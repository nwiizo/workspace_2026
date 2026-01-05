import { cookies } from "next/headers";
import * as jose from "jose";

export interface User {
  id: string;
  email: string;
  role: string;
  tenant_id?: string;
}

/**
 * ID token claims structure based on OpenID Connect Core 1.0
 * @see https://openid.net/specs/openid-connect-core-1_0.html#IDToken
 */
export interface IdTokenClaims {
  /** Subject - Identifier for the End-User */
  sub: string;
  /** Audience - Client ID(s) that this ID Token is intended for */
  aud: string | string[];
  /** Issuer - Identifier for the IdP that issued this token */
  iss: string;
  /** Expiration time (Unix timestamp) */
  exp: number;
  /** Issued at time (Unix timestamp) */
  iat: number;
  /** Time when authentication occurred */
  auth_time?: number;
  /** Access token hash for c_hash/at_hash validation */
  at_hash?: string;
  /** Email address (requires 'email' scope) */
  email?: string;
  /** Whether email has been verified */
  email_verified?: boolean;
  /** Custom claim: User role for RBAC */
  role?: string;
  /** Custom claim: Tenant ID for multi-tenancy */
  tenant_id?: string;
  [key: string]: unknown;
}

/**
 * Verify ID token signature and claims using JWKS.
 *
 * This function validates the ID token according to OpenID Connect Core 1.0 specification:
 * @see https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation
 *
 * Verification steps:
 * 1. Fetch public keys from Hydra's JWKS endpoint
 * 2. Verify the token signature using RS256/ES256
 * 3. Validate issuer matches Hydra URL
 * 4. Validate audience matches client ID
 * 5. Check token is not expired
 *
 * Ory Hydra JWKS endpoint documentation:
 * @see https://www.ory.sh/docs/hydra/reference/api#tag/jwk/operation/discoverJsonWebKeys
 *
 * @param idToken - The ID token JWT string
 * @returns Verified and decoded claims
 * @throws Error if verification fails
 */
export async function verifyIdToken(idToken: string): Promise<IdTokenClaims> {
  const hydraUrl = process.env.HYDRA_PUBLIC_URL || "http://localhost:4444";
  const clientId = process.env.NEXT_PUBLIC_CLIENT_ID || "demo-client";

  // Create JWKS remote key set with caching
  // @see https://github.com/panva/jose/blob/main/docs/functions/jwks_remote.createRemoteJWKSet.md
  const JWKS = jose.createRemoteJWKSet(
    new URL(`${hydraUrl}/.well-known/jwks.json`)
  );

  try {
    // Verify the token signature and standard claims
    // @see https://github.com/panva/jose/blob/main/docs/functions/jwt_verify.jwtVerify.md
    const { payload } = await jose.jwtVerify(idToken, JWKS, {
      issuer: hydraUrl,
      audience: clientId,
    });

    // Additional validation per OIDC spec
    const now = Math.floor(Date.now() / 1000);

    // Check issued at time is not in the future (with 60s clock skew tolerance)
    // @see https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation step 10
    if (payload.iat && (payload.iat as number) > now + 60) {
      throw new Error("ID token issued in the future");
    }

    console.log("ID token verified successfully:", {
      sub: payload.sub,
      email: payload.email,
      role: payload.role,
      iss: payload.iss,
    });

    return payload as IdTokenClaims;
  } catch (error) {
    // Handle specific jose errors with clear messages
    if (error instanceof jose.errors.JWTExpired) {
      console.error("ID token has expired");
      throw new Error("ID token has expired");
    }
    if (error instanceof jose.errors.JWTClaimValidationFailed) {
      console.error("ID token claim validation failed:", error.message);
      throw new Error(`ID token claim validation failed: ${error.message}`);
    }
    if (error instanceof jose.errors.JWSSignatureVerificationFailed) {
      console.error("ID token signature verification failed");
      throw new Error("ID token signature verification failed");
    }
    if (error instanceof jose.errors.JOSEError) {
      console.error("JOSE error:", error.message);
      throw new Error(`Token verification error: ${error.message}`);
    }
    throw error;
  }
}

/**
 * Decode ID token without verification (for debugging/logging only)
 * WARNING: Do not use this for authentication decisions!
 *
 * @param idToken - The ID token JWT string
 * @returns Decoded claims or null if parsing fails
 */
export function decodeIdTokenUnsafe(idToken: string): IdTokenClaims | null {
  try {
    const [, payload] = idToken.split(".");
    const base64 = payload.replace(/-/g, "+").replace(/_/g, "/");
    const decoded = JSON.parse(Buffer.from(base64, "base64").toString("utf-8"));
    return decoded as IdTokenClaims;
  } catch {
    return null;
  }
}

/**
 * Get the current user from the session cookie.
 * Returns null if not authenticated.
 */
export async function getCurrentUser(): Promise<User | null> {
  const cookieStore = await cookies();
  const sessionCookie = cookieStore.get("session");

  if (!sessionCookie) {
    return null;
  }

  try {
    const session = JSON.parse(atob(sessionCookie.value));
    return session.user || null;
  } catch {
    return null;
  }
}

/**
 * Check if the current user has the required role.
 */
export async function hasRole(requiredRole: string): Promise<boolean> {
  const user = await getCurrentUser();
  if (!user) return false;

  const roleHierarchy: Record<string, number> = {
    platform_admin: 4,
    tenant_admin: 3,
    tenant_staff: 2,
    customer: 1,
  };

  const userLevel = roleHierarchy[user.role] || 0;
  const requiredLevel = roleHierarchy[requiredRole] || 0;

  return userLevel >= requiredLevel;
}

/**
 * Get the login URL with redirect parameter.
 */
export function getLoginUrl(redirectTo?: string): string {
  const hydraUrl = process.env.NEXT_PUBLIC_HYDRA_URL || "http://localhost:4444";
  const clientId = process.env.NEXT_PUBLIC_CLIENT_ID || "demo-client";
  const redirectUri = process.env.NEXT_PUBLIC_REDIRECT_URI || "http://localhost:3001/callback";

  const params = new URLSearchParams({
    client_id: clientId,
    response_type: "code",
    scope: "openid profile email",
    redirect_uri: redirectUri,
    state: redirectTo || "/",
  });

  return `${hydraUrl}/oauth2/auth?${params.toString()}`;
}
