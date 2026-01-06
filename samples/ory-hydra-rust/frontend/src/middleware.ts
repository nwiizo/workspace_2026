import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

const PROTECTED_PATHS = ["/dashboard", "/incidents", "/projects", "/engineers", "/recruitment", "/leaderboard", "/tenants"];
const ADMIN_PATHS = ["/dashboard", "/projects", "/engineers", "/recruitment", "/leaderboard"];
const REPORTER_PATHS = ["/incidents"];  // Reporter can only access incidents
const PLATFORM_ADMIN_PATHS = ["/tenants"];

function getUserFromCookie(request: NextRequest): { email: string; role: string } | null {
  const userInfoCookie = request.cookies.get("user_info");
  if (!userInfoCookie || !userInfoCookie.value) return null;

  try {
    const decoded = Buffer.from(userInfoCookie.value, "base64").toString("utf-8");
    return JSON.parse(decoded);
  } catch {
    return null;
  }
}

export function middleware(request: NextRequest) {
  const pathname = request.nextUrl.pathname;
  const sessionCookie = request.cookies.get("session");
  const authTokenCookie = request.cookies.get("auth_token");

  // Check if session exists and has a valid value
  const hasValidSession = sessionCookie && sessionCookie.value && sessionCookie.value.length > 0;
  const hasValidToken = authTokenCookie && authTokenCookie.value && authTokenCookie.value.length > 0;
  const isAuthenticated = hasValidSession || hasValidToken;

  // Check if path requires authentication
  const requiresAuth = PROTECTED_PATHS.some((p) => pathname.startsWith(p));

  if (requiresAuth && !isAuthenticated) {
    const loginUrl = new URL("/api/auth/login", request.url);
    loginUrl.searchParams.set("redirect", pathname);
    return NextResponse.redirect(loginUrl);
  }

  // Check role-based access
  if (isAuthenticated) {
    const user = getUserFromCookie(request);
    const role = user?.role || "customer";

    // Platform admin paths - only platform_admin can access
    const isPlatformAdminPath = PLATFORM_ADMIN_PATHS.some((p) => pathname.startsWith(p));
    if (isPlatformAdminPath && role !== "platform_admin") {
      return NextResponse.redirect(new URL("/?error=unauthorized", request.url));
    }

    // Reporter paths - reporter, engineer, manager, platform_admin can access
    const isReporterPath = REPORTER_PATHS.some((p) => pathname.startsWith(p));
    if (isReporterPath && !["platform_admin", "manager", "engineer", "reporter"].includes(role)) {
      return NextResponse.redirect(new URL("/?error=unauthorized", request.url));
    }

    // Admin paths - platform_admin, manager, engineer can access (not reporter)
    const isAdminPath = ADMIN_PATHS.some((p) => pathname.startsWith(p));
    if (isAdminPath && !["platform_admin", "manager", "engineer"].includes(role)) {
      return NextResponse.redirect(new URL("/?error=unauthorized", request.url));
    }
  }

  // Add cache control headers to prevent caching of protected pages
  if (requiresAuth) {
    const response = NextResponse.next();
    response.headers.set("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate");
    response.headers.set("Pragma", "no-cache");
    response.headers.set("Expires", "0");
    return response;
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - api (API routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     */
    "/((?!api|_next/static|_next/image|favicon.ico).*)",
  ],
};
