# Elizabeth 1Panel App Store package

Elizabeth 的 1Panel 商店应用位于：

```text
1panel/apps/elizabeth/
```

当前商店版本为 `2.0.2`，对应：

- GitHub Release：<https://github.com/YuniqueUnic/elizabeth/releases/tag/v2.0.2>
- Docker 镜像：`yunique001/elizabeth:2.0.2`
- 支持架构：`linux/amd64`、`linux/arm64`
- 1Panel 本地应用测试目录：`/opt/1panel/resource/apps/local/elizabeth`
- 推荐提交目标：第三方应用商店
  [`okxlin/appstore`](https://github.com/okxlin/appstore)（`localApps` 分支）

> 官方 `1Panel-dev/appstore` 目前主要维护超 1 万 Star
> 的项目；社区第三方仓库由用户维护，适合 Elizabeth 这类应用。

## 生成来源

中间 AppSpec 为
[elizabeth-1panel-appspec.json](./elizabeth-1panel-appspec.json)，使用仓库中
vendored 的 1Panel AppStore skill 生成基础包：

```bash
python3 1panel/skills/appstore/scripts/generate_app_package.py \
  --spec 1panel/elizabeth-1panel-appspec.json \
  --output 1panel/apps \
  --force
```

发布时由 [prepare_appstore_release.py](./scripts/prepare_appstore_release.py) 把
`1panel/apps/elizabeth/<版本>/` 复制到输出目录，并：

- 把 compose 的 `image:` 重钉到目标版本；
- 把 `source-evidence.json` 中所有 release 域引用（镜像 tag、`/blob/v*/`、
  `/releases/tag/v*`）重钉到目标版本；
- **校验**（而非重写）根 `data.yml` 的 `document` 仍指向默认分支
  `main`。`document` 一旦钉到 `v*` 路径，每次发版都要改，且 tag 不再是 tip
  之后就会失效，所以这里选择让发布失败而不是静默改回。

生成后需要保留与 Elizabeth 官方 Compose 一致的容器加固字段：

- `init: true`
- `security_opt: [no-new-privileges:true]`
- `cap_drop: [ALL]`
- `read_only: true`
- `tmpfs: [/tmp]`
- `user: "65532:65532"`（distroless `nonroot`；由 `scripts/init.sh`
  移交目录属主）

并确认：

- 数据目录：`${APP_DATA_DIR}:/app/data`
- 文件目录：`${APP_STORAGE_DIR}:/app/storage`
- 生命周期脚本：`scripts/init.sh`（把上面两个目录 chown 给 `65532:65532` 并
  `chmod 0750`，以 root 运行）
- 溯源文件：`source-evidence.json`

根 `data.yml` 可保留
`batchInstallSupport: true`（官方商店字段，第三方商店可忽略）。

商店 Logo 由根目录透明品牌图生成，满足官方 Wiki 建议的 `180×180`、小于 `10 KB`：

```bash
magick elizabeth.logo.png -resize 180x180 -strip -colors 64 PNG8:1panel/logo.png
```

## 校验

```bash
# 版本准备与守卫（与 CI 一致）
python3 1panel/scripts/prepare_appstore_release.py \
  --version 2.0.2 \
  --output dist/1panel-appstore/apps

# 官方 1Panel 包校验器
python3 1panel/skills/appstore/scripts/validate_app_package.py \
  dist/1panel-appstore/apps/elizabeth

# Compose 展开
docker compose \
  -f dist/1panel-appstore/apps/elizabeth/2.0.2/docker-compose.yml \
  --env-file 1panel/apps/elizabeth/2.0.2/.env.sample \
  config
```

CI 另用上游 `1Panel-dev/1Panel-appstore-skills` 的 `validate-v2.sh` 以及
`okxlin/1panel-app-adapter` 的 `runtime_script_utils.py` 复核包结构、
`scripts/init.sh` 与 `source-evidence.json`。

本地安装测试时，将整个 `elizabeth` 目录复制到 1Panel
的本地应用目录，然后在应用商店更新本地应用列表。

也可参考第三方适配器：

- <https://github.com/okxlin/1panel-app-adapter>

## 自动发布 Workflow

`.github/workflows/1panel-appstore.yml` 在推送 `v*` tag 后自动执行：

1. 等待同版本 `yunique001/elizabeth` Docker 镜像发布完成。
2. 检查镜像同时包含 `linux/amd64` 与 `linux/arm64`。
3. 运行版本准备脚本、官方 1Panel package validator、Compose 展开和真实容器 smoke
   test。
4. 上传可直接安装的 1Panel package artifact。
5. 配置发布令牌后，自动从 `YuniqueUnic/thirdparty-appstore` fork 向
   `okxlin/appstore:localApps` 创建或更新 PR。

仓库需要配置 Secret `APPSTORE_GITHUB_TOKEN`。建议使用专用于
`YuniqueUnic/thirdparty-appstore` fork 的令牌；它必须能向 fork
写入内容并代表该用户向公开上游创建 Pull Request。

可选仓库变量：

| 变量                | 默认                              | 说明        |
| ------------------- | --------------------------------- | ----------- |
| `APPSTORE_UPSTREAM` | `okxlin/appstore`                 | PR 目标仓库 |
| `APPSTORE_BASE`     | `localApps`                       | 目标分支    |
| `APPSTORE_FORK`     | `YuniqueUnic/thirdparty-appstore` | 推送用 fork |

未配置 Secret 时，workflow 仍会完成生成、校验、smoke 与 artifact
上传，只跳过上游 PR。

## 官方依据

以下链接固定指向默认分支 `main`，避免每次发版都要改（逐版本的溯源证据见
`source-evidence.json`）：

- [Dockerfile.backend](https://github.com/YuniqueUnic/elizabeth/blob/main/Dockerfile.backend)
- [docker-compose.yml](https://github.com/YuniqueUnic/elizabeth/blob/main/docker-compose.yml)
- [Docker 配置](https://github.com/YuniqueUnic/elizabeth/blob/main/docker/backend/config/backend.yaml)
- [Docker 快速开始](https://github.com/YuniqueUnic/elizabeth/blob/main/docs/DOCKER_QUICK_START.md)
- [Docker 发布工作流](https://github.com/YuniqueUnic/elizabeth/blob/main/.github/workflows/docker-publish.yml)
- [1Panel 应用提交说明](https://github.com/1Panel-dev/appstore/wiki/How-to-submit-your-own-application)
- [第三方应用商店](https://github.com/okxlin/appstore)
