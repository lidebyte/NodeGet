# 主题开发

主题文件为纯静态页面，可以部署到 Cloudflare Pages / GitHub Pages / 腾讯 EdgeOne 等静态文件托管服务。

建议将受限 Token 直接写入到某个配置文件（如 `config.json`）。

可以在 Dashboard Token 管理处生成预设的 Visitor Token。

参考配置：

```json
{
    "site_name":"",
    "site_log":"",
    "theme_name":"",
    "theme_repo":"",
    "theme_config":{

    },
    "site_tokens":[
        {
            "name":"master server node 1",
            "websocket":"wss://HOST1",
            "token":"Your Token"
        },
        {
            "name":"master server node 2",
            "websocket":"wss://HOST2",
            "token":"Your Token"
        }
    ]
}
```

建议为每个静态主题的 GitHub 增加「部署到 Cloudflare」的便捷部署按钮。

主题的推荐使用方式是：

Fork 到自己的 GitHub，修改 `config.json` 然后点击部署到 Cloudflare。

或者下载 GitHub Code Zip，修改 `config.json`，手动上传。

## 优秀主题汇总

占位下，优秀的主题会被整理一份到这里。