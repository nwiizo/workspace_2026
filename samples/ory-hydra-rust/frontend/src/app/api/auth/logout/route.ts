import { cookies } from "next/headers";
import { NextResponse } from "next/server";

export async function GET() {
  const cookieStore = await cookies();
  const baseUrl = process.env.NEXT_PUBLIC_BASE_URL || "http://localhost:3001";
  const hydraAdminUrl = process.env.HYDRA_ADMIN_URL || "http://localhost:4445";
  const hydraPublicUrl = process.env.HYDRA_PUBLIC_URL || "http://localhost:4444";
  const clientId = process.env.NEXT_PUBLIC_CLIENT_ID || "demo-client";
  const clientSecret = process.env.CLIENT_SECRET || "demo-secret";

  // Get session info before clearing cookies
  const sessionCookie = cookieStore.get("session");
  let accessToken: string | null = null;
  let userId: string | null = null;

  if (sessionCookie?.value) {
    try {
      const session = JSON.parse(
        Buffer.from(sessionCookie.value, "base64").toString("utf-8")
      );
      accessToken = session.access_token;
      // Try to get user ID from the session
      if (session.user?.id) {
        userId = session.user.id;
      }
    } catch {
      // Ignore parse errors
    }
  }

  // Clear all session cookies first
  cookieStore.set("session", "", {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 0,
  });

  cookieStore.set("auth_token", "", {
    httpOnly: false,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 0,
  });

  cookieStore.set("user_info", "", {
    httpOnly: false,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 0,
  });

  // Revoke the access token at Hydra Public API
  if (accessToken) {
    try {
      await fetch(`${hydraPublicUrl}/oauth2/revoke`, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: new URLSearchParams({
          token: accessToken,
          client_id: clientId,
          client_secret: clientSecret,
        }),
      });
      console.log("Token revoked successfully");
    } catch (err) {
      console.error("Failed to revoke token:", err);
    }
  }

  // If we have the access token, introspect it to get the subject (user ID)
  if (accessToken && !userId) {
    try {
      const introspectRes = await fetch(`${hydraAdminUrl}/admin/oauth2/introspect`, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: new URLSearchParams({
          token: accessToken,
        }),
      });
      if (introspectRes.ok) {
        const data = await introspectRes.json();
        if (data.sub) {
          userId = data.sub;
        }
      }
    } catch (err) {
      console.error("Failed to introspect token:", err);
    }
  }

  // Delete Hydra login and consent sessions via Admin API
  if (userId) {
    try {
      // Delete consent sessions
      await fetch(
        `${hydraAdminUrl}/admin/oauth2/auth/sessions/consent?subject=${encodeURIComponent(userId)}&all=true`,
        { method: "DELETE" }
      );
      console.log("Consent sessions deleted for user:", userId);
    } catch (err) {
      console.error("Failed to delete consent sessions:", err);
    }

    try {
      // Delete login sessions
      await fetch(
        `${hydraAdminUrl}/admin/oauth2/auth/sessions/login?subject=${encodeURIComponent(userId)}`,
        { method: "DELETE" }
      );
      console.log("Login sessions deleted for user:", userId);
    } catch (err) {
      console.error("Failed to delete login sessions:", err);
    }
  }

  // Redirect to home page
  return NextResponse.redirect(new URL("/", baseUrl));
}
