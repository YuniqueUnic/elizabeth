/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    ignoreBuildErrors: false,
  },
  images: {
    unoptimized: true,
  },
  allowedDevOrigins: ["local-origin.dev", "*.local-origin.dev"],
  // Enable standalone output for Docker deployment
  output: "standalone",

  async headers() {
    return [
      {
        source: "/",
        headers: [
          {
            key: "Cache-Control",
            value: "no-store, max-age=0, must-revalidate",
          },
        ],
      },
    ];
  },

  // API rewrites for backend proxy - Next.js 16 compatible version
  async rewrites() {
    const target = process.env.INTERNAL_API_URL;

    console.log('Next.js: INTERNAL_API_URL from env:', target);

    if (!target) {
      console.warn('WARNING: INTERNAL_API_URL is not set. API rewrites will not work.');
      return [];
    }

    return [
      {
        source: '/api/v1/:path*',
        destination: `${target}/:path*`,
      },
    ];
  },
};

export default nextConfig;
