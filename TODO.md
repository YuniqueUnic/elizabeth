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
