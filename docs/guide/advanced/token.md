# Token 机制

NodeGet 是一个完全的前后端分离的项目，每个操作都有对应的 API，而几乎所有的 API 都有相关的细粒度权限 Token。

在面板上使用时，可能觉察不到 Token 的存在，因为在默认使用具有完整权限的 SuperToken。

而一旦涉及开发插件、分享 Js Worker、与第三方系统交互等等场景，都会用到细粒度 Token 机制。

NodeGet 允许你只把一部分权限开放出去，仅允许授权的行为，比如：

- 对于公开探针页面，可以只开放监控数据的读取，补充开放 Ping 数据的读取
- 对于 Agent 节点，仅开放对应 Agent UUID 的上报权限
- 对于插件，只开放允许的权限，比如 HTTP Request / Exec 权限等

详细的 Token 机制，阅读 [API](/api/token/) 部分。