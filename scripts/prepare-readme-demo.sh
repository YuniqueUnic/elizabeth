#!/usr/bin/env bash
# 一次性素材脚本：为 README 截图准备演示房间（中英双语房间 + admin/editor token）。
# 输出 /tmp/eliz-shots-env.json 供 capture-readme-shots.mjs 使用。
set -euo pipefail
BASE="${1:-http://127.0.0.1:4092}"
BOOT="dev-admin-token"

api() { curl -s --max-time 10 "$@"; }

mint() { # room role -> full token json
  api -X POST "$BASE/api/v1/rooms/$1/tokens" -H "Content-Type: application/json" \
    -H "X-Elizabeth-Admin-Token: $BOOT" \
    -d "{\"role\":\"$2\",\"with_refresh_token\":true}"
}

ensure_room() { # room -> create if missing
  api -X POST "$BASE/api/v1/rooms/$1" -H "Content-Type: application/json" -d '{}' >/dev/null
}

post_msg() { # room token text
  api -X POST "$BASE/api/v1/rooms/$1/messages" -H "Content-Type: application/json" \
    -H "Authorization: Bearer $2" -d "$(python3 -c 'import json,sys; print(json.dumps({"text": sys.argv[1]}))' "$3")" >/dev/null
}

upload() { # room token name file mime
  local RES SIZE
  SIZE=$(stat -f%z "$3")
  RES=$(api -X POST "$BASE/api/v1/rooms/$1/contents/prepare" -H "Content-Type: application/json" \
    -H "Authorization: Bearer $2" -d "{\"files\":[{\"name\":\"$3\",\"size\":$SIZE,\"mime\":\"$4\"}]}" \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['reservation_id'])")
  api -X POST "$BASE/api/v1/rooms/$1/contents?reservation_id=$RES" -H "Authorization: Bearer $2" \
    -F "file=@$3;type=$4" >/dev/null
}

post_url() { # room token url name desc
  api -X POST "$BASE/api/v1/rooms/$1/contents/url" -H "Content-Type: application/json" \
    -H "Authorization: Bearer $2" \
    -d "$(python3 -c 'import json,sys; print(json.dumps({"url": sys.argv[1], "name": sys.argv[2], "description": sys.argv[3]}))' "$3" "$4" "$5")" >/dev/null
}

ZH=readme-demo
EN=readme-demo-en

ensure_room "$ZH"; ensure_room "$EN"

ZH_ADMIN=$(mint "$ZH" admin)
EN_ADMIN=$(mint "$EN" admin)
ZH_EDITOR=$(mint "$ZH" editor)
EN_EDITOR=$(mint "$EN" editor)

zh_tok=$(echo "$ZH_ADMIN" | python3 -c "import json,sys; print(json.load(sys.stdin)['token'])")
en_tok=$(echo "$EN_ADMIN" | python3 -c "import json,sys; print(json.load(sys.stdin)['token'])")

# 中文房间内容（幂等：仅在 0 条消息时写入）
COUNT=$(api "$BASE/api/v1/rooms/$ZH/messages?limit=1" -H "Authorization: Bearer $zh_tok" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('items', [])))")
if [ "$COUNT" = "0" ]; then
  post_msg "$ZH" "$zh_tok" '# 欢迎来到 Elizabeth 协作房间

这里演示一个典型的共享工作流：

- **实时消息** — Markdown、代码高亮、即时同步
- **文件共享** — 拖拽上传，支持图片 / PDF / 文本预览
- **链接收藏** — 相关资料一键贴入，自动生成预览

右侧面板管理文件，左侧边栏集中了身份、分享与常用配置。'
  post_msg "$ZH" "$zh_tok" '部署配置刚刚更新好了，关键片段如下：

```yaml
room:
  default_age: 2h
  max_size: 50MiB
  default_role: reader
```

`JWT_SECRET` 记得换成稳定的 32+ 字符密钥。'
  post_msg "$ZH" "$zh_tok" '**本周共享清单**

1. 设计稿 `poster-draft.png`
2. 会议纪要 `meeting-notes.md`
3. 项目主页链接（见右侧）

有问题直接在房间里 @ 我。'
fi

# 英文房间内容
EN_COUNT=$(api "$BASE/api/v1/rooms/$EN/messages?limit=1" -H "Authorization: Bearer $en_tok" | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('items', [])))")
if [ "$EN_COUNT" = "0" ]; then
  post_msg "$EN" "$en_tok" '# Welcome to your Elizabeth room

A typical sharing workflow in one place:

- **Realtime messages** — Markdown, syntax highlighting, instant sync
- **File sharing** — drag & drop upload with image / PDF / text preview
- **Link collection** — paste a URL and get a rich preview automatically

Manage files in the right panel; identity, sharing, and everyday settings live in the left sidebar.'
  post_msg "$EN" "$en_tok" 'Deployment config just landed, key snippet:

```yaml
room:
  default_age: 2h
  max_size: 50MiB
  default_role: reader
```

Remember to use a stable 32+ character `JWT_SECRET`.'
  post_msg "$EN" "$en_tok" '**Sharing list for this week**

1. Design draft `poster-draft.png`
2. Meeting notes `meeting-notes.md`
3. Project homepage link (see the right panel)

Drop questions in the room and mention me.'
fi

# 文件与链接（按房间各传一份）
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cp "$REPO_ROOT/color/assets/icon.png" /tmp/poster-draft.png
printf '# Meeting notes - week 42\n\n- Permission model approved: admin / editor / reader\n- Editor identity codes capped at 10, issued by admin\n- File download policy + access codes ship next week\n\n## TODO\n\n- [ ] Release v1.7 images\n- [ ] Extend e2e coverage for visibility\n' > /tmp/meeting-notes-en.md
printf '# 会议纪要 · 第 42 周\n\n- 房间权限模型评审通过：admin / editor / reader 三级\n- editor 身份码上限 10 个，由 admin 统一签发\n- 文件下载策略与访问码保护下周灰度\n\n## 待办\n\n- [ ] 发布 v1.7 镜像\n- [ ] 补充 e2e 覆盖显隐能力\n' > /tmp/meeting-notes-zh.md

for ROOM in "$ZH" "$EN"; do
  TOK=$( [ "$ROOM" = "$ZH" ] && echo "$zh_tok" || echo "$en_tok" )
  N=$(api "$BASE/api/v1/rooms/$ROOM/contents" -H "Authorization: Bearer $TOK" | python3 -c "import json,sys; print(sum(1 for c in json.load(sys.stdin) if c['content_type']['type'] != 'text'))")
  if [ "$N" = "0" ]; then
    upload "$ROOM" "$TOK" poster-draft.png /tmp/poster-draft.png image/png
    if [ "$ROOM" = "$ZH" ]; then
      upload "$ROOM" "$TOK" meeting-notes.md /tmp/meeting-notes-zh.md text/markdown
      post_url "$ROOM" "$TOK" "https://www.rust-lang.org" "Rust 官网" "后端技术栈主页"
    else
      upload "$ROOM" "$TOK" meeting-notes.md /tmp/meeting-notes-en.md text/markdown
      post_url "$ROOM" "$TOK" "https://www.rust-lang.org" "Rust homepage" "Backend tech stack"
    fi
  fi
done

python3 - "$ZH_ADMIN" "$ZH_EDITOR" "$EN_ADMIN" "$EN_EDITOR" <<'PY'
import json, sys
def slim(raw):
    d = json.loads(raw)
    return {
        "token": d["token"],
        "expiresAt": d.get("expires_at"),
        "refreshToken": d.get("refresh_token"),
        "capabilities": d.get("capabilities"),
        "roleKey": (d.get("claims") or {}).get("role"),
    }
out = {
    "zh": {"admin": slim(sys.argv[1]), "editor": slim(sys.argv[2])},
    "en": {"admin": slim(sys.argv[3]), "editor": slim(sys.argv[4])},
}
with open("/tmp/eliz-shots-env.json", "w") as f:
    json.dump(out, f)
print("tokens written; zh admin caps:", len(out["zh"]["admin"]["capabilities"] or []))
PY
