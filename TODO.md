TODO:

1. 修复 CI/CD 的 release, version bump (blocked by
   https://github.com/release-plz/release-plz/issues/2479)
2. 加入 postgresql 数据库到后端
3. 清理程序代码，清理没用的东西
4. 重新检查 README.md, 并且修正。

ISSUES:

1. 前端的创建房间的 Option 密码却少一个显示/隐藏密码功能 (done)
   1. 如果显示了密码，那么就只有一个密码框
   2. 如果没有显示密码，那么就两个密码框 (两个用于确保用户输入的密码一致)
2. 后端 AppConfig.LoggingConfig.level 解析存在问题，始终为 "off",
   需要找到原因，并且修复
3. 通过 https://box.yunique.top 访问前端时，创建房间时不能成功
   1. 是由于前端的请求来自 https, 而前端和后端都是 http 交互。
   2. 前端和后端是通过 docker-compose 启动的一组容器。
   3. 需要找到解决办法

---

通过 https://box.yunique.top 访问前端时，创建房间时不能成功
点击创建房间时，会报如下错误，并且也不会跳转至创建房间的 page.:

请你直接访问 https://box.yunique.top (使用 chrome-devtools mcp)
吧，然后找到问题所在，

Application error: a client-side exception has occurred while loading
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

□ 分析与修复前端 chunk 404：确认 build 与 serve
流程、调整配置/脚本确保生产模式提供静态资源，并验证 https://box.yunique.top
创建房间成功。 □ 重构 ConfigManager/cfg_service
的配置优先级、文件创建/保存逻辑，确保 logging.level
等字段正确解析并符合优先级规则，同时只在需要时写默认文件。 □
补充/更新自动化测试（单元/集成）覆盖优先级、默认文件创建等场景，并本地运行测试验证。
□ 整理验证结果并输出说明、局限与后续建议。
