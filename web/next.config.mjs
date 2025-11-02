/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  allowedDevOrigins: ["local-origin.dev", "*.local-origin.dev"],
  // Enable standalone output for Docker deployment
  output: "standalone",
};

export default nextConfig;
