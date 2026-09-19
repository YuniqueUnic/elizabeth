---
name: 1panel-appstore-package
description: Validate, generate and submit a 1Panel v2 third-party app-store package (okxlin/appstore style). Use when working with `apps/<key>/{data.yml, <version>/, source-evidence.json}`, `validate-v2.sh`, or when a 1Panel submission is blocked by validation failures, CVEs, a version-pinned `document` link, or a license/redistribution conflict.
agent_created: true
license: AGPL-3.0-only
allowed-tools:
disable: false
---

# 1Panel v2 商店包：校验与提交

## 何时用

- 要给 1Panel 第三方商店生成或更新应用包
- 商店包校验不过，或 PR 被维护者以「CVE 阻断 / `document` 钉版本 / 许可证冲突」打回
- 需要在**没有 bash 4+ 的 macOS** 上跑官方校验器

## 包结构（v2）

```
apps/<key>/
  data.yml               # 商店元数据（含 document 字段 + additionalProperties）
  README.md  README_en.md
  logo.png
  source-evidence.json   # 溯源证据（发布时重钉版本引用）
  <version>/             # 每个版本一个目录，**历史版本目录要保留**
    data.yml  docker-compose.yml  .env.sample
    scripts/init.sh      # 生命周期脚本，由 adapter 的 finalize 生成
    data/  storage/      # 只放 .gitkeep
```

要点：

- 上游 `apps/<key>/` **保留多个历史版本目录**。**只追加新版本，不要删旧的**。
- `container_name: ${CONTAINER_NAME}` **不能加引号**。
- `scripts/init.sh` 用 `BASH_SOURCE` 定位 `ROOT_DIR`，**与 cwd 无关**，所以从任意目录调用都安全。

## 拿到校验器

官方校验器不在应用仓库里，是独立的 adapter：

```bash
git clone --depth 1 https://github.com/okxlin/1panel-app-adapter /tmp/adapter
```

关键脚本：`scripts/validate-v2.sh`、`scripts/source_evidence.py`、`scripts/package_contract.py`、
`scripts/runtime_script_utils.py`（`--finalize-lifecycle --dir-owner` 生成/校验 `init.sh`）。

## macOS 上的两个硬障碍

### 1. `validate-v2.sh` 需要 bash 4+

用了 `mapfile`，且 `$( ... <<'PY' ... PY )` 的**收尾括号单独成行** —— 后者是 **bash 3.2 的解析缺陷**，
脚本会报 `unexpected EOF while looking for matching '"'`。

**`BASH_ENV` 注入 `mapfile` 垫片救不了解析错误**，必须有真正的 bash 4+。macOS 自带 3.2，
`/opt/homebrew/bin/bash` 默认不存在。可行解是容器（`debian:trixie-slim` 含 bash 5）：

```bash
docker run --rm -v /tmp/adapter:/adapter -v /abs/pkg-out:/pkg debian:trixie-slim bash -c \
  "apt-get update -qq && apt-get install -y -qq python3-yaml && \
   bash /adapter/scripts/validate-v2.sh --dir /pkg/<key> --version <ver> --submission-profile third-party"
```

- 容器内**没有 docker CLI** → 会打印 `docker not available; skipped docker compose config validation`
  并跳过该检查。**必须在宿主另行补跑** `docker compose --env-file .env.sample config --quiet`。
- 结果的判定看 `SUMMARY: fail=… warn=… info=…` 与 `PASS:`/`FAIL`，**不要只看退出码**。
  **不要用管道吞退出码**（`... | tail` 会把失败伪装成成功）。

### 2. Python 侧需要 PyYAML

`source_evidence.py` / `package_contract.py` 要 `import yaml`，而系统 `python3` 与 Homebrew 的
`python3` 常常都没有。用一个带 PyYAML 的解释器，或临时建 venv（`python3 -c 'import yaml'` 自检）。

好处：**Python 侧校验不需要 bash 4**，可以单独快速迭代：

```bash
PYTHONPATH=/tmp/adapter/scripts <python-with-yaml> /tmp/adapter/scripts/source_evidence.py \
  <pkg>/source-evidence.json --artifact-root <pkg> \
  --compose <pkg>/<ver>/docker-compose.yml --env-file <pkg>/<ver>/.env.sample \
  --version-name <ver> [--require-delivery]
```

## 档位与常见缺口

| 档位 | 参数 | 用途 |
| --- | --- | --- |
| third-party | `--submission-profile third-party`（默认） | **第三方提交走这个** |
| strict delivery | `--strict-store --i18n-mode strict --source-evidence-mode required --require-delivery-evidence` | 官方交付，要求最严 |

- `--strict-store` + `--source-evidence-mode required` **会隐式打开** `--require-delivery-evidence`。
- 开了 delivery 档后，**包内不允许存在 `source-evidence.json`**，要用 `--source-evidence <外部路径>`。
- 大量 `warn` 是**正常**的（多语言 label/description 缺失，`fa`/`lo` 等小语种常年缺），
  已合并的基线也是 100+ warn。**只看 `fail` 是不是 0**。
- delivery 档专属缺口：`images[].digest`（**必须等镜像真的发布后才存在，无法提前填**）、
  `redistributionEvidence`（官方档范畴）。第三方档不要求，不必为此阻塞。

## `source-evidence.json` 契约

```jsonc
{
  "repository": "https://…",           // 必填 https
  "dockerDocs": "https://…/blob/vX/…", // 必填 https
  "composeFile": "https://…/blob/vX/docker-compose.yml",
  "licenseEvidence": { "spdx": "AGPL-3.0-only", "url": "https://…/blob/vX/LICENSE" },
  "logoEvidence":    { "source": "https://… | bundled:<相对路径>", "license": "…", "sha256": "<64hex>" },
  "redistributionEvidence": { "status": "verified|unresolved", "assets": [...], "materials": [...] },
  "notes": ["…"]
}
```

- `licenseEvidence` 只要 `spdx` 或 `url` 之一；`spdx` **不能**命中占位黑名单
  （`n/a`、`na`、`none`、`placeholder`、`tbd`、`todo`、`unknown`、`unspecified`、`unverified`）。
- 若同时给了 `logoEvidence` 且 `redistributionEvidence.assets` 里有 `logo.png`，
  两者的 `source` / `license` / `sha256` **必须逐字段一致**。
- `redistributionEvidence.status = "verified"` 时 `assets` 不能为空，且每个 asset 的
  `license` 不能是占位值、`source` 不能是 `unverified:`。
- 若给了 `images`，其 `version` 必须等于 compose 的版本目录名、`service` 必须是 compose 服务名。

## 版本引用重钉 vs 稳定链接（维护者最在意的一点）

- **`data.yml` 的 `document` 必须指向默认分支**，不能钉 `vX.Y.Z`，否则每次发版都要改这个字段。
  **正确做法是加守卫而不是静默改写** —— 静默改写正是这个字段最初被钉死的成因：

  ```python
  def assert_stable_document_url(path: Path) -> None:
      match = DOCUMENT_RE.search(path.read_text(encoding="utf-8"))
      if match is None or match.group(2) != STABLE_DOCUMENT_URL:
          raise ValueError("`document` must point at the default branch")
  ```

- 反之，**`source-evidence.json` 里的版本引用应当重钉**（它是发布证据，要指向确切的 tag/镜像）：
  用正则替换 `<registry>/<image>:<ver>`、`/blob/v<ver>/`、`/releases/tag/v<ver>` 三类模式。
  注意 `/blob/main/` **不会被 `/blob/v[0-9]…/` 模式命中**，所以稳定链接能安全共存。

## 提交方式（不要在应用仓库里手搓 PR）

应用仓库的 CI 用 `peter-evans/create-pull-request` 自动提交：

- `base: localApps`、**固定 head 分支名**（如 `app/<key>`）、`push-to-fork: <fork>`、
  `add-paths: apps/<key>`
- 流程是「检出上游 `localApps` → `cp -R` 覆盖 `apps/<key>` → 提交」

推论：

- **不会新建 PR**（同一 head 分支 → 更新已有 PR）。
- 上游 PR 显示 `CONFLICTING` 时，**重跑即自动消除**（分支重建在当前上游之上），不需要手工 rebase。
- CI 里通常有一步「等镜像发布」轮询（`docker buildx imagetools inspect`），所以 tag 触发时
  「镜像构建」与「商店包提交」并行也不会竞态。

## 常见阻断与处置

| 现象 | 根因 | 处置 |
| --- | --- | --- |
| CVE 阻断 | 基础镜像的用户态软件包（典型：`perl-base`） | 换 **distroless** 运行镜像（按 digest 钉定），而不是追点版本 |
| 许可证冲突 | 自定义/非 OSI 许可（如「All rights reserved」式私有协议） | 换 OSI 认可许可，并在 `source-evidence.json` 补 `licenseEvidence` |
| `document` 被钉版本 | 发布脚本静默改写 | 改成**守卫**（不匹配就 fail），见上 |
| `container_name` 报错 | 加了引号 | 去掉引号 |
| `init.sh` 拒绝运行 | 祖先链属主不是 uid 0，**或 mode 被 group/other 写** | 见下节 |
| 容器以非 root 跑但目录写不进 | bind mount 在 Linux 保留宿主属主 | `init.sh` 里 chown 到容器 uid/gid；Docker Desktop 会虚拟化属主，无需 chown |
| 健康检查用了 `curl` | distroless 无 shell/curl | 把 `HEALTHCHECK` 以 **exec 形式**烘焙进镜像，调用自带子命令 |

### `init.sh` 的两类前置条件（都踩过）

`verify_trusted_root_chain` 从版本根目录**一路向上走到 `/`**，逐层要求：

1. **属主是 uid 0**（`stat -c '%u:%g'`）；
2. **mode 不含 group/other 写位**（`mode & 0022 == 0`）。

第 2 条容易被忽略：`mkdir` 会继承 umask，**CI runner 的 umask 未必是 0022**，于是
`sudo mkdir -p /opt/<app>-smoke` 出来的目录可能带 group 写位，脚本只报
`unsafe version root chain permissions: <path>` 而**不报 mode**，很难定位。
⇒ 建目录后**显式 `chmod 0755`**，并把 `stat -c '%u:%g:%a'` 打进日志。
⇒ 测试根目录必须建在 root 拥有的路径下（如 `/opt/...`），不能建在 runner 的临时目录里。

## 新增容器断言前必须先量时序

`curl` 能通 **≠** `docker inspect .State.Health.Status` 已是 `healthy`。健康检查按自己的
`interval` 走，实测：**端口 2s 应答，首次 `healthy` 判定 6s**。所以「curl 成功后立刻断言
`healthy`」**必然读到 `starting` 而失败**。

正确写法是**有界轮询**等到 `healthy`，超时再 dump `docker inspect --format '{{json .State.Health}}'`。
**新加任何容器断言都要本地先跑一遍量时序**，不能靠「看起来对」。

## 自查清单

1. `docker compose --env-file .env.sample config --quiet` 在包目录通过（宿主）
2. `validate-v2.sh --submission-profile third-party` → `fail=0`
3. 镜像扫描 0 CRITICAL / 0 HIGH
4. `document` 指向默认分支，且有守卫防回归
5. `data.yml` 的 `envKey` 与 compose 的 `environment` 键**闭合**（无缺失、无多余）
6. 历史版本目录未被删除
7. 新增/修改的容器断言已本地实测过时序
