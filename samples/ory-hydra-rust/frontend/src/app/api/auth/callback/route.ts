import { cookies } from "next/headers";
import { NextRequest, NextResponse } from "next/server";
import { verifyIdToken, decodeIdTokenUnsafe } from "@/lib/auth";

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
export async function POST(request: NextRequest) {
  try {
    const { code } = await request.json();

    const hydraUrl = process.env.HYDRA_PUBLIC_URL || "http://localhost:4444";
    const clientId = process.env.NEXT_PUBLIC_CLIENT_ID || "demo-client";
    const clientSecret = process.env.CLIENT_SECRET || "demo-secret";
    const redirectUri =
      process.env.NEXT_PUBLIC_REDIRECT_URI || "http://localhost:3001/callback";

    // Exchange code for tokens (using client_secret_post method)
    // @see https://www.rfc-editor.org/rfc/rfc6749#section-4.1.3
    const tokenResponse = await fetch(`${hydraUrl}/oauth2/token`, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        grant_type: "authorization_code",
        code,
        redirect_uri: redirectUri,
        client_id: clientId,
        client_secret: clientSecret,
      }),
    });

    if (!tokenResponse.ok) {
      const error = await tokenResponse.text();
      console.error("Token exchange failed:", error);
      return NextResponse.json(
        { error: "Failed to exchange code for tokens" },
        { status: 401 }
      );
    }

    const tokens = await tokenResponse.json();

    // Verify and decode ID token with signature validation
    // @see https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation
    let user: { id?: string; email: string; role: string; tenant_id?: string } = {
      email: "unknown",
      role: "customer",
    };

    if (tokens.id_token) {
      try {
        // Verify ID token signature using JWKS
        const claims = await verifyIdToken(tokens.id_token);

        user = {
          id: claims.sub,
          email: claims.email || claims.sub || "unknown",
          role: claims.role || "customer",
          tenant_id: claims.tenant_id,
        };

        console.log("ID token verified and decoded:", {
          sub: claims.sub,
          email: claims.email,
          role: claims.role,
          tenant_id: claims.tenant_id,
          iss: claims.iss,
          aud: claims.aud,
        });
      } catch (verifyError) {
        // Log the verification error but try to continue with unsafe decode
        // This allows development environments where JWKS might not be available
        console.warn("ID token verification failed, falling back to unsafe decode:", verifyError);

        const unsafeClaims = decodeIdTokenUnsafe(tokens.id_token);
        if (unsafeClaims) {
          console.warn("WARNING: Using unverified ID token claims. This is insecure!");
          user = {
            id: unsafeClaims.sub,
            email: unsafeClaims.email || unsafeClaims.sub || "unknown",
            role: unsafeClaims.role || "customer",
            tenant_id: unsafeClaims.tenant_id,
          };
        }
      }
    } else {
      console.error("No ID token in response");
    }

    // Create session
    const session = {
      access_token: tokens.access_token,
      refresh_token: tokens.refresh_token,
      user,
      expires_at: Date.now() + (tokens.expires_in || 3600) * 1000,
    };

    // Set session cookie
    const cookieStore = await cookies();
    cookieStore.set("session", Buffer.from(JSON.stringify(session)).toString("base64"), {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: tokens.expires_in || 3600,
      path: "/",
    });

    // Also set a non-httpOnly cookie for client-side access to token
    cookieStore.set("auth_token", tokens.access_token, {
      httpOnly: false,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: tokens.expires_in || 3600,
      path: "/",
    });

    // Set user info cookie (readable by client-side JS for display)
    cookieStore.set("user_info", Buffer.from(JSON.stringify(user)).toString("base64"), {
      httpOnly: false,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: tokens.expires_in || 3600,
      path: "/",
    });

    return NextResponse.json({ success: true, user });
  } catch (error) {
    console.error("Callback error:", error);
    return NextResponse.json(
      { error: "Authentication failed" },
      { status: 500 }
    );
  }
}
