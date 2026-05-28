/**
 * Next.js App Router server page wrapper for [roomName] dynamic route.
 *
 * In `output: 'export'` (static export) mode, Next.js requires all dynamic
 * segments to either provide `generateStaticParams()` or be explicitly
 * marked as `dynamicParams: true` (which is unsupported in static export).
 *
 * Solution: export an empty `generateStaticParams()` so Next.js is satisfied
 * and does NOT pre-render any specific room at build time. At runtime, the
 * Axum SPA fallback serves `index.html` for ALL unknown paths, and the
 * client-side `usePathname()` hook correctly reads the real URL to obtain
 * the room name — no hydration mismatch, no build-time placeholder bleed.
 */
import RoomClientPage from "./RoomClientPage";

// 为 Next.js 静态导出提供占位符路由：生成 web/out/_/index.html
// Axum fallback 会将所有 /[roomName] 请求路由到这个预生成的 HTML 文件。
// 客户端 usePathname() 从真实浏览器 URL 读取实际房间名，不受占位符影响。
export function generateStaticParams() {
  return [{ roomName: "_" }];
}

export default function RoomPage() {
  return <RoomClientPage />;
}
