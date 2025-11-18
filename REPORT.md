# Elizabeth Frontend/Backend Investigation Report

请你直接访问 https://box.yunique.top 吧，然后找到问题所在，利用 chrome-devtools
mcp 来使用 remote debug chrome 直接访问 https://box.yunique.top后 看到了
elizabeth 的前端，创建房间/进入房间的功能按钮，点击这两个按钮均没有反应，并且
consoleerror 提示如下：Application error: a client-side exception has occurred
while loading box.yunique.top (see the browser console for more information).

Failed to load resource: the server responded with a status of 404 ()Understand
this error turbopack-5d0e2dd1b9f3ea1a.js:1 Uncaught Error: Failed to load chunk
/_next/static/chunks/b71e021ebc4e57fd.js from module 64893 at
turbopack-5d0e2dd1b9f3ea1a.js:1:5708

https://box.yunique.top 则是真实的环境了.(其背后是本地的 docker
容器，请你仔细的核查并且找到问题所在，并且修复)

你可以使用 /Users/unic/dev/projs/rs/elizabeth/manage_services.sh
来控制本地的前后端程序启动，停止，以及 log 查看 注意：

1. manage_services.sh 控制的是本地的前端/后端程序
2. 而 https://box.yunique.top 则是真实的环境，是访问的 docker 容器。

## 这两个是不一样的，我希望你能够利用这两个来辅助你解决 https://box.yunique.top 中的问题.,

同时还有个问题：
后端的配置解析问题，用户传入的配置文件/或者是从默认位置读取到的配置文件并未成功。
尤其是后端 AppConfig.LoggingConfig.level 解析存在问题，始终为 "off",
需要找到原因，并且修复。并且需要写测试，确保 配置解析顺序和逻辑正确
配置文件解析顺序和逻辑正确 priority: 命令行参数 > 程序环境变量 > 配置文件值 >
程序默认配置值

1. 程序启动时，如果用户没有指定配置文件 (--config-file/-c),
   那么就是使用默认配置文件
2. 如果默认配置文件不存在，那么就使用命令行>程序环境变量>配置文件值>程序默认配置值，并且创建/写入配置文件。
   - 如果默认配置文件不存在，创建默认配置文件
   - 如果用户还用了命令行参数，以及环境变量，那么就还是按照 priority
     优先级进行解析和覆盖。
   - 并且会更新默认配置文件的内容。
3. 如果用户指定了配置文件，那么就使用用户指定的配置文件，但是不覆盖默认配置文件。
   - 如果用户指定的配置文件不存在，直接报错即可
   - 如果用户不仅指定了配置文件，还用了命令行参数，以及环境变量，那么就还是按照
     priority 优先级进行解析和覆盖。
   - 并且也不会更改用户指定的配置文件的内容.(仅仅读取即可)

## 关键的一些内容

my deploy structure:

    user-web
        ^
        |

---

https://box.yunqiue.top/ ^ | | my server vps nginx ^ |
reverse-proxy(localhost:52033) ^ | frp-server(52033) ^ |

---

        |                   my local pc
    frp-client (localhost:4001 -> 52033)
        ^
        |
    docker-compose (bind 4001:4001)
        ^
        |

frontend (4001) + backend (4092)

frp-server 机器 你可以使用 ssh
功能来进行检索。`ssh root@121.41.3.22 -p 10022 -i
~/.ssh/id_ed25519_aliyun`

- 然后这个是 frp-server 上的 nginx 目录：/www/server/nginx
- 这里是 /www/wwwroot/box.yunique.top
- 然后这里是 frps 的配置文件：/opt/1panel/apps/frps/frps/data/frps.toml
- 并且这个 frps 实际是通过 docker 部署的，1Panel-frps-x40N, 这是容器名

请你查看 Dockerfile.frontend docker-compose.yml Dockerfile.backend
manage_services.sh，然后修复线上 https://box.yunique.top
上的问题，并且测试通过。测试请使用 chrome-devtools mcp。请你仔细，认真的
思考和测试，并且修复，确保功能正确。

利用 chrome-devtools mcp 来使用 remote debug chrome 直接访问
https://box.yunique.top后 看到了 elizabeth
的前端，创建房间/进入房间的功能按钮，点击这两个按钮均没有反应，并且 consoleerror
提示如下：Application error: a client-side exception has occurred while loading
box.yunique.top (see the browser console for more information).

Failed to load resource: the server responded with a status of 404 ()Understand
this error turbopack-5d0e2dd1b9f3ea1a.js:1 Uncaught Error: Failed to load chunk
/_next/static/chunks/b71e021ebc4e57fd.js from module 64893 at
turbopack-5d0e2dd1b9f3ea1a.js:1:5708

https://box.yunique.top 则是真实的环境了.(其背后是本地的 docker
容器，请你仔细的核查并且找到问题所在，并且修复)

你可以使用 /Users/unic/dev/projs/rs/elizabeth/manage_services.sh
来控制本地的前后端程序启动，停止，以及 log 查看 注意：

1. manage_services.sh 控制的是本地的前端/后端程序
2. 而 https://box.yunique.top 则是真实的环境，是访问的 docker 容器。

这两个是不一样的，我希望你能够利用这两个来辅助你解决 https://box.yunique.top
中的问题.,

## 时间线与关键动作

1. **本地构建与脚本修复**
   - `manage_services.sh` 现在会在 `start_frontend` 中执行 `pnpm build`，并在
     `.next/standalone/.next/static` 与 `public` 之间同步静态资源，再用
     `node .next/standalone/server.js` 启动本地前端。这样本地再也不会出现 dev
     chunk 随进程消失的问题。
   - 在仓库内把 `@radix-ui/react-slot`、`@uiw/react-markdown-preview` 明确加入
     `web/package.json`，并同步更新 `pnpm-lock.yaml`，保证
     Docker/本地同一套依赖。
2. **Docker 镜像调整**
   - `Dockerfile.frontend` 改为使用 pnpm，build
     阶段同样执行静态资源同步；`.dockerignore` 允许 `pnpm-lock.yaml`
     进入构建上下文，以确保依赖可复现。
   - 重新 `docker compose build frontend && docker compose up -d frontend`
     后，容器内 `/app/.next/static/chunks` 与 `.next/standalone` 均含最新
     chunk；`curl https://box.yunique.top/_next/static/chunks/f2bbca832a68e7ed.js`
     可直接 200。
3. **后端配置体系**
   - `configrs::ConfigManager` 新增 `file_exists`；`cfg_service::init`
     只在默认路径缺失时落盘，避免覆盖。
   - `apply_env!` 宏集中处理 `.env`
     中的程序级变量（LOG_LEVEL、JWT__、MIDDLEWARE__ 等），优先级为 CLI >
     程序环境 > 配置文件 > 默认。
   - 新增并通过测试
     `program_env_overrides_apply_before_cli`、`cli_env_file_priority_respected`。

## 当前线上状态

- 通过 Chrome DevTools 在 `https://box.yunique.top/?_ts=...` 访问时，一切脚本
  200（或 304）。`curl` 到对应 chunk 也是 200。
- **根路径 `/` 仍返回旧的 HTML**，其中包含早期构建的 chunk 名（`297b1930…`,
  `b71e021…` 等），浏览器自然会请求不存在的文件 → 404/502。
- 观察响应头：`x-nextjs-cache: HIT`、`cache-control: s-maxage=31536000`，说明
  **远端节点（frp 反向代理、信任链上的 nginx 或 CDN）缓存了旧 HTML**。本地重启
  Docker / rebuild 只影响新的 `/` 请求路径 `/?_ts=timestamp`，对已经缓存的 `/`
  无法生效。
- 该缓存来自 frp 服务器 `box.yunique.top`：
  - Nginx 配置 `/www/server/panel/vhost/nginx/box.yunique.top.conf` +
    `proxy/.../1c2a…conf` 指向 frp remote `127.0.0.1:52033`，并设置 `expires 1m`
    与 `add_header X-Cache`，但 HTML 仍被上游（可能是 Next.js 内部或某
    CDN）缓存。
  - frp 通过 `frps` (remote) + `frpc` (本机
    `/Users/unic/dev/frp/config.toml`，映射 local 4001 → remote 52033)
    传输。`frps` Web API 显示 `elizabeth` 对应 remotePort 52033 正在线。

## 存在的问题 & 可能原因

1. **静态 HTML 被缓存**
   - `/` 的响应里
     `x-nextjs-prerender: 1`，`cache-control: s-maxage=31536000`，说明 Next.js
     本身将首页预渲染并 emit 了极长的缓存头。若没有显式吊销，该 HTML
     会驻留在上游代理或 CDN 中。
   - `_ts` 参数的一致性：本地应用在 `/_ts=` 情况下才返回最新 HTML，这是因为
     `page.tsx` 会立即 `redirect('/?_ts=Date.now())` 使 URL 带上
     query，从而绕过缓存。但现在 `https://box.yunique.top/` 直接命中旧缓存，永远
     404 chunk。
2. **frpc 与 Docker 并存**
   - 当前 `frpc` 在本机以 CLI 方式运行，配置文件 `~/dev/frp/config.toml`
     直接指向 `localhost:4001`。这意味着只要本机 4001 有任何服务（哪怕是 dev
     server），frp 都会把它暴露给线上，容易造成“线上跑 dev、线下跑 prod”的错觉。
   - 即使我们停止 docker-compose，`manage_services.sh start frontend` 又会在
     4001 起一个本地进程，frp 即刻对外暴露，而真实 Docker
     容器可能并未运行。这种切换非常危险。

## 建议与下一步

1. **缓存层处理**
   - 在盒子服务器上执行 `nginx -s reload` 或直接清除
     `/www/wwwroot/box.yunique.top`
     是否存在的静态文件（当前目录看上去像早年部署的 SPA，确认后可备份再清理）。
   - 配合 `manage_services`，当 `_ts` 逻辑确定没必要时，可让 `/` 直接返回
     HTML，同时把 `s-maxage` 调低或在上游配置 `proxy_no_cache`。
   - 若使用 CDN/Cloudflare，需要在控制台 Purge 对应域名，否则 `/` 永远是旧版本。
2. **统一出口**
   - 不要同时运行 docker 容器和本地 `manage_services` 前端。建议让 frpc 指向
     Docker 容器的地址（例如 `host.docker.internal:4001` 或 `localhost:4001`
     但确保只有容器在监听），并把 `manage_services.sh` 仅用于调试（端口改成 4100
     之类）。
   - 或在 frpc config 中增加 `subdomain`/`remote_port` 区分 dev 与
     prod，避免互相覆盖。
3. **进一步记录**
   - 若确实需要 `_ts`，请在 README 或运维手册中写明缘由，并说明如何在 nginx 层
     purge 缓存。
   - 建议把 `REPORT.md` 纳入版本控制（若合适）以便后续成员快速了解上下文。

## 关键文件索引

- `Dockerfile.frontend`：lines 8-74（pnpm 构建 + 静态同步）。
- `docker-compose.yml`：`frontend` service 指向 4001 端口；和
  `.dockerignore`（lines 70-78）确保锁文件存在。
- `manage_services.sh`：lines 11-131 控制本地生产模式启动。
- `web/app/page.tsx`：新的入口组件，若无 `_ts` 则 redirect，直接回退到 `/`
  会触发缓存。
- `/Users/unic/dev/frp/config.toml`：本机 frpc -> 远端 52033 映射。
- `/www/server/panel/vhost/nginx/box.yunique.top.conf` + `proxy/...conf`：远端
  nginx 入口，决定 static 缓存。

希望这份报告能帮助下一位同事迅速了解：目前线上 404 并非 Docker 或代码的 chunk
问题，而是远端缓存 & frp 映射未更新所致。清空缓存/重建静态目录或调整 `_ts`
处理后，再配合严格的部署流程（只允许容器监听 4001 并通过 frp
暴露）才能彻底关闭这个问题。
