import { redirect } from "next/navigation";
import { NextRequest } from "next/server";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const redirectTo = searchParams.get("redirect") || "/";
  const bffBaseUrl = process.env.NEXT_PUBLIC_BFF_BASE_URL || "http://localhost:3000";

  redirect(`${bffBaseUrl}/api/bff/login?redirect=${encodeURIComponent(redirectTo)}`);
}
