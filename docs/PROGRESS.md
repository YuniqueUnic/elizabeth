我希望前端和后端“一体化”, 从而就能解决下面的 Mixed Content 问题。
或者说还有什么简单且易用的解决方案？请你仔细思考并给出解决方案。

## deploy structure

        user
        ^
        |
    https://box.yunique.top(https://frpserver:443:http://frpserver:52033)
        ^
        |

---

        |
        nginx
        ^
        |
    reverse proxy(http://frpserver:52033 : http://frpclient:4001)
        ^
        |
    frp server (52033)
        ^
        |

---

        |
    frp client (4001)
        ^
        |
    docker bind (4001)
        ^
        |

^----------------^ | | frontend backend (4001:4001) (-:4092)

## 访问流程

1. **用户访问**：用户访问 https://box.yunique.top，nginx 将请求转发到 frp server
   的 52033 端口。
2. **frp server**：frp server 将请求转发到 frp client 的 4001 端口。
3. **frp client**：frp client 将请求转发到 docker 容器的 4001 端口。
4. **docker 容器**：docker 容器将请求转发到 4001 端口。
5. docker 容器内有：frontend(4001), backend(4092)
6. frontend 调用 backend api, 从 frontend:4001 访问 backend:4092.

## 当前的问题

在上述的访问流程中，用户端是通过 https 来发起的请求，而实际内部是 http
的。以至于有 浏览器的 mixed-content
请求。浏览器会拒绝这些请求，导致用户无法访问到后端服务。Mixed Content: The page
at 'https://box.yunique.top/' was loaded over HTTPS, but requested an insecure
resource 'http://elizabeth-backend:4092/api/v1/rooms/hello?password=abc123'.
This request has been blocked; the content must be served over HTTPS.

## 相关文件

@docker-compose.yml the frontend and backend in the same docker-compose bu not
the same docker container. @nginx_elizabeth.conf this the frontend nginx
configuration. @niginx.conf this is the nginx main configuration.

And if you encounter any issues, please use the web search or related tools to
help you solve this problem correctly.

## 2025-11-07 缓存问题修复进度

- 删除首页 `_ts` 重定向，使用 Next.js `connection()` 与 `revalidate = 0`
  强制动态渲染，避免构建期产出长期缓存的 HTML。
- 在 `next.config.mjs` 中为根路径追加
  `Cache-Control: no-store, max-age=0, must-revalidate`
  响应头，阻止上游代理继续缓存旧页面。
- 本地执行 `pnpm build` 与 `cargo check` 均通过，确认前后端构建流程正常。
- 建议生产环境部署后对 frp/nginx 做一次缓存清理或重载，彻底清除历史 HTML。

## 2025-11-08 首页返回旧 HTML 的修复补充

- 移除 `next/server` 的 `connection()` 调用，避免 Next.js 在生产模式下写入 `_ts`
  重定向逻辑。
- 新增 `proxy.ts`（原 middleware），在代理层统一删除 `_ts`
  查询参数，确保根路径始终命中最新 HTML。Next 16 对 `middleware`
  文件名已废弃，升级到 `proxy` 以适配 Node Runtime。
- 重新构建并发布 Docker 前端镜像，确认本地 `node .next/standalone/server.js`
  与容器 `http://localhost:4001/` 请求均返回最新脚本清单。
- 线上 `https://box.yunique.top` 首屏 HTML 脚本指向最新 chunk（如
  `1dfffbc0d9c48c1c.js` 等），浏览器页面加载正常，按钮交互恢复。

## 2025-11-08 Mixed Content 防护增强

- 调整 `web/lib/utils/api.ts` 的
  `buildClientUrl`，在浏览器环境检测协议降级与同域情况，强制返回相对路径，彻底消除
  HTTPS 页面向 HTTP 源发起请求的可能。
- 清理遗留的 `web/middleware.ts`，仅保留 Next.js 16 推荐的
  `proxy.ts`，避免构建时出现“Both middleware and proxy detected”错误。
- 本地复验 `pnpm build` 与 `cargo check`
  均通过，确认前后端构建链路稳定；构建日志记录 `INTERNAL_API_URL`
  警告，部署前需在环境变量中补足该值以启用 API rewrite。
- 建议上线前同步更新部署脚本/`.env`，确保
  `NEXT_PUBLIC_API_URL=/api/v1`、`INTERNAL_API_URL=http://elizabeth-backend:4092/api/v1`，并重新构建前端镜像，使新逻辑生效。
