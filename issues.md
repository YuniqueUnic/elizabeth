当前的问题：前端的问题：

0. 当一个房间具有删除权限时，那么该房间的所有权限都应该是开启状态 [和 1 相关联]
1. 而当房间的删除权限被关闭，并保存之后，后续对房间设置的设置的修改都是不被允许的..
   [未完成，当前的情况是，房间只有密码设置能够成功，.]
2. 当前房间无法在房间设置的房间密码处设置房间密码，并且保存..需要修复。 [完成]
3. 当房间的 share 权限被关闭之后，用户无法直接通过 [完成]
   http://localhost:4001/[room_name] 的方式直接尝试访问房间。
   1. 当房间的 share 权限处于开启时，用户可以直接通过
      http://localhost:4001/[room_name] 的方式直接尝试访问房间。
      1. 如果房间存在，则尝试访问
      2. 如果房间不存在，则直接创建该房间
   2. 当房间的 share 权限被关闭之后，用户无法通过
      http://localhost:4001/[room_name] 的方式直接尝试访问房间。
      1. 而是通过 http://localhost:4001/[room_name_uuid] 的方式尝试访问房间。
         1. 也就是后端对应的 slug 从 room_name 改为 room_name_uuid
         2. 而这个 uuid 随机生成的，并且确保 room_name_uuid 是唯一的。
      2. 在前端操作时，当房间还含有
         删除权限时，说明该房间还允许配置房间设置，那么用户就可以配置房间的分享权限。
         1. 如果用户这时，关闭 share 权限，并且保存配置。
         2. 那么该用户就会立马从 http://localhost:4001/[room_name] 导向新的房间
            http://localhost:4001/[room_name_uuid].
4. 前端的文件管理出的 upload icon button 并没有实际的功能..
   需要确保也能和底部的上传一样[完成]
5. 文件管理处的文件，当文件名字过长时，会让文件 item
   长度过长，而导致显示不全..需要让名字过长的文件名被 truncated. [依旧存在，可能
   css 也影响了]
6. 当用户点击 messagebubble 上的删除按钮。或者 topbar 上的删除按钮时，还处于
   ensure 的 dialog 阶段时，底部的 toast 就弹出说 以及成功删除...
   这里的逻辑是不对的.. 因为 ensure dialog 阶段，底部的 toast
   就不应该弹出。底部的 toast 应该在 ensure dialog 阶段结束之后才弹出。[完成]
7. settings 中的 enter or ctrl+enter 键，发送消息的逻辑并未生效...
   需要确保该设置有效。[完成]
8. topbar 的 帮助按钮弹出来的 help dialog, 内容很多，却没有 vertical scroll
   功能，导致底部内容看不到...需要修复。[完成]
9. 容量使用 和 房间占用 的信息展示不太对。单位是 MB, 但是实际的数字却是 bytes
   的实际数字.. 需要修复。确保是 MB[完成]
10.

当前的问题：前端的问题：

0. 当一个房间具有删除权限时，那么该房间的所有权限都应该是开启状态 [和 1 相关联]
1. 而当房间的删除权限被关闭，并保存之后，后续对房间设置的设置的修改都是不被允许的..
   [未完成，当前的情况是，房间只有密码设置能够成功，.后续的权限设置都无法成功..我们的设计应该是，只要删除权限被开启时，那么该房间的所有权限都应该是开启状态，也就是删除权限==管理员，有编辑权限，则应允许用户输入消息，上传文件，,并且保存，有预览权限，则允许用户查看文件，下载文件，查看消息等权限]
2. 文件管理处的文件，当文件名字过长时，会让文件 item
   长度过长，而导致显示不全..需要让名字过长的文件名被 truncated. [依旧存在，可能
   css 也影响了]

后端的问题：

1. 从前端上传到 room 中的 content 为文件时，由于上传的文件自带一个文件名，
   而我们当前还会在文件名之前添加一个 prefix(uuid),
   所以导致上传的文件名会变成：uuid_filename.ext 的格式，
   从而使得，文件名过长了... 正确的行为是，应该创建一个 room_id
   的文件夹，然后将上传的文件保存在文件夹中，保持原本的文件名。后续该 room
   内的文件查询/删除/下载等逻辑都应该遵循这个逻辑

现在前后端基本都有问题，但是我不太了解当前的项目具体实现和逻辑了，请你仔细阅读前端代码，然后查看关联的后端代码，然后整理一个各个功能的的逻辑呢？
比如：

1. 前端的权限管理逻辑，和后端的集成逻辑，以及后端的处理逻辑
2. 前端的文件管理逻辑，和后端的集成逻辑，以及后端的处理逻辑
3. 前端的房间设置的逻辑，和后端的集成逻辑，以及后端的处理逻辑

我需要只知道这些真实的实现逻辑，从而才能知道当前实现的问题出在哪里，以及和预期的设计的偏差，从而才能
指出问题所在，并且给出修复建议。

please fix those tests. I recommend you manual test firstly using browser tools
(or chrome-devtools), and during manually testing. record each operations and
items id (or path). and update the
/Users/unic/dev/projs/rs/elizabeth/web/e2e/selectors/html-selectors.ts
definition of web-items. and then correct the related playwright tests.

you can run the above operation as a loop flow till all tests are manually
passed, and all UI automation tests are passed.

if you encounter problems or issues, please check the code from
backend/frontend. and you also can use sqlite to retrieve sqlite database which
can aid you locate the issue.(if you want to reset database, please run just
reset-db)

and if you want to kill/restart/start/status backend/frontend service, please
use manage_services.sh to control

- backend: /Users/unic/dev/projs/rs/elizabeth/crates/board
- frontend: /Users/unic/dev/projs/rs/elizabeth/web
- database: /Users/unic/dev/projs/rs/elizabeth/app.db
- service manage script: /Users/unic/dev/projs/rs/elizabeth/manage_services.sh

you always stuck during exec bash cmd. recommend to use desktop-commander to do
such things which has timeout parameter can avoid such situation.

后端的问题：

1. 从前端上传到 room 中的 content 为文件时，由于上传的文件自带一个文件名，
   而我们当前还会在文件名之前添加一个 prefix(uuid),
   所以导致上传的文件名会变成：uuid_filename.ext 的格式，
   从而使得，文件名过长了... 正确的行为是，应该创建一个 room_id
   的文件夹，然后将上传的文件保存在文件夹中，保持原本的文件名。后续该 room
   内的文件查询/删除/下载等逻辑都应该遵循这个逻辑

2.

3. 访问前端页面 http://localhost:4001/ 时：会进入入口 page 页面
   - 创建房间 (用户输入房间名称)
   - 加入房间 (而当我输入还未存在的房间时，用户 enter
     一个不存在的房间，应该提示暂无该房间，而当前的逻辑是直接就创建了一个新的房间...是不对的.)

这两个功能按钮功能应该合并为一个 进入房间
按钮即可。如果房间存在，那么就尝试进去，如果房间有密码，那么就让用户输入密码。
