# NodeGet Worker

如果你用过 Cloudflare Workers，那么对这个功能肯定很熟悉，NodeGet 的 Js Worker 可以粗略理解是其本地部署版本。

Js Worker 是 NodeGet 实现很多自定义行为的基础，NodeGet 将所有 API 操作映射为内部函数，并提供了定时执行、绑定 HTTP 路由、API 调用执行等功能。

NodeGet Js Worker 功能的前后端都经过了多次打磨与优化，可以提供畅快的开发及使用体验。