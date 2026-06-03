import { NextResponse } from "next/server";

export async function GET() {
  const bffBaseUrl = process.env.NEXT_PUBLIC_BFF_BASE_URL || "http://localhost:3000";
  return NextResponse.redirect(`${bffBaseUrl}/api/bff/logout`);
}
