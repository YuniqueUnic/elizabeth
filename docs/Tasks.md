## MVP

- 启动 web-server
- RESTFUL API
  - 示例网站：www.unicshare.com
  - GET www.unicshare.com/\[name\] 如果存在该 name 的 room 则加载
    - 如果 room - locked by password, 需要用户输入 password
    - 进入 room
  - GET www.unicshare.com/\[name\] 如果不存在该 name 的 room 则新建该 room,
    但是该 room 暂时只存在于本地 (该怎么做呢)
    - 用户可以为该 share room 设置 password, expire-time, times-of-looking,
      添加文本 (配置/自动检测 language-syntax/highlight-syntax), 添加图片 (支持
      wasm 客户端压缩后再上传，限制总图片大小
      50m，直接像相册一样提供瀑布流预览/也支持选中两个对比), 添加文件
      (限制总文件大小 100m?).
    - 用户添加完毕之后，点击保存按钮，POST 用户添加的内容到该 room 中。
  - 因此 POST 这是具有创建/删除 room 的功能，还具有上传文本/文件的功能

## 其它

- 给 configrs 添加上 log dep, 然后在合适的地方添加上对应的 log
