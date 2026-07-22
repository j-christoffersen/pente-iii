import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  async rewrites() {
    return [{ source: "/game", destination: "/game/index.html" }];
  },
};

export default nextConfig;
