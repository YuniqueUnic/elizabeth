# CLI / curl 速查（命令行完整工作流）

纯命令行（如 SSH 到远程服务器）从建房到上传/下载的完整序列。所有命令均已在
本地实例上逐条实测走通；示例使用占位符，不含任何真实凭据。

```bash
export BASE="http://localhost:4092"   # 按部署调整；注意不含 /api/v1 后缀
command -v jq >/dev/null || echo "以下示例依赖 jq"
```

错误响应统一为 JSON envelope，脚本据此判断成败：

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation error: ...",
    "status": 400
  }
}
```

建议 `curl` 一律加 `-f`（非 2xx 直接失败退出），配合 `set -e` 或 `&&` 串联。

## 1. 创建房间（获得 admin token）

```bash
CREATE=$(curl -sS -f -X POST "$BASE/api/v1/rooms/my-room" \
  -H 'Content-Type: application/json' -d '{}')
TOKEN=$(echo "$CREATE" | jq -r '.token')            # admin JWT（Bearer 使用）
echo "$CREATE" | jq -r '.identity_code'             # admin 身份码，仅此一次回显
```

- 带密码建房：`-d '{"password":"xxx"}'`。
- 指定 admin
  身份码（重复执行可复建同码房间）：`-d '{"admin_identity_code":"..."}'`。
- 房间已存在返回 `409 CONFLICT`。

## 2. 发送文本消息

```bash
curl -sS -f -X POST "$BASE/api/v1/rooms/my-room/messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"text":"hello from the command line"}' | jq -r '.message.text'
```

## 3. 单命令上传文件（curl -T）

`PUT /api/v1/rooms/{name}/files/{filename}`：一条命令完成预留、传输与落盘，
等价于"prepare + multipart upload"两阶段流程。

```bash
printf '# CLI demo\n' > report.md
UPLOAD=$(curl -sS -f -T report.md \
  -H "Authorization: Bearer $TOKEN" \
  "$BASE/api/v1/rooms/my-room/files/report.md")
DOWNLOAD_URL=$(echo "$UPLOAD" | jq -r '.uploaded[0].download_url')
# => /api/v1/contents/2
```

- 响应 `uploaded[0].download_url` 为可直接使用的相对下载路径；`current_size`
  为房间当前占用。
- 请求头必须带正确的 `Content-Length`（`curl -T` 自动设置）；服务端校验实际
  字节数，不符即 400 并释放预留额度。
- 与普通上传一样受房间容量（413）与房间级文件类型策略（400，消息前缀
  `File type not allowed by room policy:`）约束。
- 文件名不允许路径分隔符、`.`/`..` 或控制字符（400 `Invalid file name`）。

## 4. 下载

```bash
curl -sS -f -o report.md "$BASE$DOWNLOAD_URL?token=$TOKEN"
```

- `download_url` 不携带任何凭据；下载必须附加房间 token（query 参数 `?token=`
  便于 `wget`/浏览器，`curl` 也可用 `-H "Authorization: Bearer $TOKEN"`）。
- 需要下载票据的文件见第 6 节；token 只证明房间身份，票据策略不受影响。

wget 等价写法：

```bash
wget -O report.md "$BASE$DOWNLOAD_URL?token=$TOKEN"
```

## 5. 签发与兑换身份码（editor / reader）

管理员签发一个 editor 身份码（角色必须存在于房间角色集）：

```bash
curl -sS -f -X POST "$BASE/api/v1/rooms/my-room/identity-codes" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"code":"editor-code-demo-2026","role":"editor","expires_in_secs":86400}'
```

凭身份码兑换 JWT（无需房间密码）：

```bash
EDITOR_TOKEN=$(curl -sS -f -X POST "$BASE/api/v1/rooms/my-room/identity-codes/redeem" \
  -H 'Content-Type: application/json' \
  -d '{"code":"editor-code-demo-2026"}' | jq -r '.token')
curl -sS -f -T report.md -H "Authorization: Bearer $EDITOR_TOKEN" \
  "$BASE/api/v1/rooms/my-room/files/editor-upload.md" | jq -r '.uploaded[0].file_name'
```

- 身份码仅保存 Argon2 哈希，明文只在创建/重置响应中出现一次。
- `expires_in_secs` 仅对非 admin 角色生效（60 秒 ~ 10
  年，且不超过房间过期时间）。

## 6. 受保护下载（下载策略 + 下载票据）

文件默认无下载策略：凭房间 token 即可下载（第 4 节）。对单个文件设置策略后，
下载需要额外的、短时有效的下载票据：

```bash
CONTENT_ID=2

# 设置可复用策略（off=无策略 / reusable=凭码可多次 / one_time=凭码一次）
curl -sS -f -X PUT "$BASE/api/v1/rooms/my-room/contents/$CONTENT_ID/policy" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"mode":"reusable","max_downloads":10}'

# 生成访问码
CODE=$(curl -sS -f -X POST "$BASE/api/v1/rooms/my-room/contents/$CONTENT_ID/policy/generate-codes" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"count":1,"is_reusable":true}' | jq -r '.codes[0]')

# 用访问码兑换下载票据（120 秒有效；下载者也必须是房间成员）
TICKET=$(curl -sS -f -X POST "$BASE/api/v1/rooms/my-room/contents/$CONTENT_ID/redeem" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "{\"code\":\"$CODE\"}" | jq -r '.ticket')

# 携带票据下载
curl -sS -f -o report.md "$BASE$DOWNLOAD_URL?token=$TOKEN&ticket=$TICKET"
```

## 7. 常用错误码

| code                                         | status | 典型原因                                                  |
| -------------------------------------------- | ------ | --------------------------------------------------------- |
| `VALIDATION_ERROR`                           | 400    | 文件名不合法、Content-Length 不符、文件类型被房间策略拒绝 |
| `AUTHENTICATION_FAILED` / `TOKEN_ERROR`      | 401    | 缺 token、token 无效或过期                                |
| `PERMISSION_DENIED` / `AUTHORIZATION_FAILED` | 403    | 角色无对应能力、下载票据/访问码无效                       |
| `NOT_FOUND`                                  | 404    | 房间/内容不存在                                           |
| `PAYLOAD_TOO_LARGE`                          | 413    | 超出房间容量或请求体上限                                  |
| `CONFLICT`                                   | 409    | 房间名已存在                                              |

完整 API 见 `API_GUIDE.md` / `API_GUIDE_FULL.md`；交互式文档 `/api/v1/scalar`。
