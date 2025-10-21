https://github.com/oustn/cloudflare-drop?tab=readme-ov-file
https://github.com/99percentpeople/weblink p2p WebRTC
https://github.com/szabodanika/microbin/tree/master 很接近我想要的了，但是
license 和 url 问题

https://github.com/blenderskool/blaze p2p file sharing

## MVP

1. 不能是随机的 url, 需要像 webnote
2. 支持文本/图片/文件
3. 支持代码片段渲染
4. 支持设置文件分享的密码/过期时间/最大查看次数/最大下载次数/是否允许编辑等
5. 支持图片/文件 (PDF...) 等等预览
6. 支持图片对比 (左右滑动对比)
7. 接入 cloudflare R2 等 s3.

尽量让所有的东西都在一页上显示 单一可执行文件完成部署

这里是构建创建 room 中 content 的 handler 的地方 该过程需要有如下流程：

1. 该请求必须拿到 room 签发的一个凭证 (如 jwt), 然后使用该凭证去发起该 room 的
   content 的一系列操作
   1. 这个 jwt 只有在用户成功进入该 room 时才会签发，也就是如果该 room
      有一个密码，那么只有用户成功进入该 room 才能获取到该凭证
   2. 该 jwt 的有效期应该小于该 room 的有效期
   3. 房间不存在时，jwt 也应该失效
2. 用户发起的 content 相关请求需要带上该凭证，然后使用该凭证去发起该 room
   的内容相关的操作
3. 服务拿到用户的请求信息后，会先检查 jwt,
   1. 服务拿到用户的请求信息后，会先检查 jwt，并解析出该 jwt 所对应的房间信息
      (权限。当前所占 size) 以及 新文件 size 等等信息。
   2. 如果这个 jwt 是有效的，有对应的 room, 那么就会开始检查 权限，检查允许的总
      size < 当前 size + 新文件 size. 等等需要检查的内容。
   3. 如果检查通过，那么就会开始处理该请求
      (比如获取文件列表，上传文件等等)，并返回结果。
4. 用户得到服务端返回的结果 content handler 应该有这些方法/接口
5. 获取某个房间下的文件列表
6. 上传文件 (s) 到某个房间
7. 删除某个房间下的某些文件
8. 下载某个房间下的某些文件 room handler
   也应该更新，增加一个签发凭证，验证凭证，删除凭证，获取凭证等等的常用方法/接口。
   我们的这个程序是没有用户的概念的，只有 room
   的概念，能够进入房间的都是能够对房间进行处理的
   (但是房间的权限是由最开始创建的人配置的权限。后续的人是没有该权限的)
   相关文件： /Users/unic/dev/projs/rs/elizabeth/crates/board/src/handlers
   /Users/unic/dev/projs/rs/elizabeth/crates/board/src/repository 此外关于 sqlx,
   当前项目的常用构建等等操作，请你参考
   /Users/unic/dev/projs/rs/elizabeth/justfile 请你最后实现和完成之后，参考
   /Users/unic/dev/projs/rs/elizabeth/docs/great-blog/how-2-write-blog.md
   写一篇博客，描述你的需求，分析，实现过程，痛点等等的技术性文章。输出到
   /Users/unic/dev/projs/rs/elizabeth/docs 中

当前的实现中：
对应的文件：/Users/unic/dev/projs/rs/elizabeth/crates/board/src/handlers/content.rs

1. 上传 room 中的 contents 时，是将 content 埃个上传到 server 上，然后再 check
   该文件的 size 是否 符合要求：即 new-file-size + current-file-size <=
   max-file-size. 这样的做法是不对。上传文件 (s)
   时，应该在用户客户端就做好文件大小获取和计算。所以上传到 server
   的阶段也应该分为：
1. 客户端：用户选择要上传的文件/文件集
1. 客户端：计算本次需要上传的文件总大小
1. 客户端发起一次上传检验的请求：告诉 server
   本次上传的文件总大小，相关的文件信息 (name,size), 并且得到 server
   端的回复，如果允许，那么这说明还有足够空间。可以上传。并且，这个时候，server
   对应的 room 的 current_file_size
   也应该更新，进行空间占用。从而避免并发的问题。如果 10s
   内，没有发起真实的上传文件/文件集请求，那么 current_file_size 应该被 server
   端进行回滚.(理论上来说，这种情况不太会出现，
   因为客户端也是我们编写的前端代码.) 因此还需要一张临时的表用来存储 room_id,
   对应的 jwt, 该 jwt 的 new_file_size, last_check_time. 用于跟踪该 jwt
   的上传文件相关信息，方便回滚。

1. server
   返回了允许上传的内容之后，客户端则可以执行真正的上传，将文件/文件集上传到
   server.
1. server 接收到上传文件/文件集的请求，验证 jwt,
   验证相关文件是否和第三步一致。一致之后，则正式开始传输了。

此外还有一个实现未完成：
对应的文件：/Users/unic/dev/projs/rs/elizabeth/crates/board/src/models/room/permission.rs
RoomPermission 还有个字段 const SHARE = 1 << 2;
这个字段的含义：是否允许用户直接通过 http://domain.com/room_name 的方式访问 room
(当前我们构建的是后端 api, 所以 api 接口不一致) 但是前端会调用对应的后端 api.)
如果 允许的话，那么这个字段的含义就是：允许用户直接通过
http://domain.com/room_name 的方式访问对应的 room 如果
不允许的话，那么这个字段的含义就是：不允许用户直接通过
http://domain.com/room_name 的方式访问对应的 room 用户需要访问的该 SHARE 为
false 的 room 的话，就需要我们通过我们给出的链接来访问对应的 room.
http://domain.com/room_name_[uuid] 也就是 room_name_[uuid], room_name +
随机的一个 uuid 拼接重构构成的新的 room name. 如果该 room_name 存在，
那么就需要重新再生成 room_name + uuid 直到唯一 (但是重复的概率很低).

然后还有这些问题，也请修复和改进

1. **权限自定义接口**：目前房间默认全权限，后续新增接口允许第一个创建该房间的人房间的后续权限；
   - 成功创建房间后，允许设置权限；
   - 如果第一个用户配置了房间的权限为只读 (那么后续的人进入房间后，就只能只读)
   - 如果第一个用户配置了房间的权限为读写，那么后续的人进入房间后，就能增改查
     room content
   - 如果第一个用户配置了房间的权限为分享，读写，那么后续的人就能通过
     http://domain.com/room_name 的方式访问房间，然后，就能增改查 room content
   - 如果第一个用户配置了房间的权限为分享，读写，删除，那么后续进入房间的人就也有上面的能力，也能立马将当前房间删除。

2. **token 生命周期管理**：引入刷新机制或一次性
   token，避免长期有效带来的安全风险；
