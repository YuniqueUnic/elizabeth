/** @type {import('next').NextConfig} */
const isProd = process.env.NODE_ENV === "production";

const nextConfig = {
  typescript: {
    ignoreBuildErrors: false,
  },
  images: {
    unoptimized: true,
  },
  allowedDevOrigins: ["local-origin.dev", "*.local-origin.dev"],

  // 生产模式：静态导出 (web/out/)，由 Rust 二进制 rust-embed 打包并 serve
  // 开发模式：standalone，保留 Next.js dev server + API rewrites 代理
  output: isProd ? "export" : "standalone",

  // 仅本地开发时：将 /api/v1/* 代理到后端（4092 端口）
  ...(isProd
    ? {}
    : {
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

        async rewrites() {
          const target =
            process.env.INTERNAL_API_URL ||
            process.env.NEXT_PUBLIC_BACKEND_URL ||
            "http://127.0.0.1:4092/api/v1";

          console.log("Next.js: API proxy target:", target);

          return [
            {
              source: "/api/v1/:path*",
              destination: `${target}/:path*`,
            },
          ];
        },
      }),
};

export default nextConfig;
