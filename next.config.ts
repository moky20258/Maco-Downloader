import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",
  images: {
    unoptimized: true,
  },
  // 确保 API routes 不被包含（我们使用 Tauri commands）
  webpack: (config) => {
    return config;
  },
};

export default nextConfig;
