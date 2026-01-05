import { headers } from "next/headers";

const RESERVED_SUBDOMAINS = ["admin", "api", "auth", "www", "localhost"];

/**
 * Get the current tenant slug from the request host header.
 * Returns null for reserved subdomains or when no tenant is identified.
 */
export async function getCurrentTenant(): Promise<string | null> {
  const headersList = await headers();
  const host = headersList.get("host") || "";

  // Extract subdomain from host
  const subdomain = host.split(".")[0];

  // Handle localhost with port
  if (subdomain.includes(":")) {
    return null;
  }

  // Check if it's a reserved subdomain
  if (RESERVED_SUBDOMAINS.includes(subdomain.toLowerCase())) {
    return null;
  }

  // Return the tenant slug
  return subdomain || null;
}

/**
 * Check if the current request is for a tenant-specific route.
 */
export async function isTenantRoute(): Promise<boolean> {
  const tenant = await getCurrentTenant();
  return tenant !== null;
}

/**
 * Get the base URL for the current tenant.
 */
export async function getTenantBaseUrl(): Promise<string> {
  const headersList = await headers();
  const host = headersList.get("host") || "";
  const protocol = process.env.NODE_ENV === "production" ? "https" : "http";
  return `${protocol}://${host}`;
}
