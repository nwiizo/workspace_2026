import { redirect } from "next/navigation";
import { NextRequest } from "next/server";
import { randomBytes } from "crypto";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const redirectTo = searchParams.get("redirect") || "/";

  const hydraUrl = process.env.NEXT_PUBLIC_HYDRA_URL || "http://localhost:4444";
  const clientId = process.env.NEXT_PUBLIC_CLIENT_ID || "demo-client";
  const redirectUri =
    process.env.NEXT_PUBLIC_REDIRECT_URI || "http://localhost:3001/callback";

  // Generate a secure state parameter (includes random bytes + redirect path)
  const randomState = randomBytes(16).toString("hex");
  const state = `${randomState}:${Buffer.from(redirectTo).toString("base64")}`;

  const params = new URLSearchParams({
    client_id: clientId,
    response_type: "code",
    scope: "openid profile email",
    redirect_uri: redirectUri,
    state: state,
  });

  redirect(`${hydraUrl}/oauth2/auth?${params.toString()}`);
}
