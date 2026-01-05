import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Use standalone output for Docker deployment
  output: "standalone",
};

export default nextConfig;
