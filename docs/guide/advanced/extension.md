# NodeGet 插件机制

NodeGet 是一个具有细粒度 Token 模型的软件，对于不同的操作定义了详细的权限限制。

为了方便基于 NodeGet 的 API 和 Token 设计的辅助程序，官方提供一个将程序嵌入到 Dashboard 上的方法。

这个工作其实并不复杂，原则上相当于在 Server 上实现了一个静态文件储存器。

前端通过 `iframe` 加载对应的储存器，然后支持注册到 Dashboard 的路由上即可。

## 先说场景

在介绍具体的插件工作原理前，先说一个具体的插件的目录结构和文件内容，有一个大体的印象。

### 文件结构

```
demo-extension
├── app.json
├── readme.md
├── resources
│   ├── assets
│   │   ├── icon.svg
│   │   ├── main.js
│   │   ├── route-icon.svg
│   │   └── style.css
│   └── index.html
└── worker.js
```

### app.json

```json
{
    "name":"my-extenstion",
    "icon":"assets/icon.svg",
    "routes":[
        {
            "type":"node",
            "name":"extension-test-1",
            "icon":"assets/route-icon.svg",
            "entry":"index.html"
        },
        {
            "type":"global",
            "name":"extension-test-2",
            "icon":"assets/route-icon.svg",
            "entry":"index.html"
        }
    ],
    "limits":[
        // 这里由用户输入，Token 所需要的权限列表
    ],
    // 非关键字段，仅记录用
    "version": "1.0",
    "description": "Just an extension demo.",
    "extension_version": 1,
    "author":"",
    "repository":"",
    "homepage":"",
    "license":"",
}
```

## 插件安装流程介绍

插件的安装流程很简单，简述如下：

- 用户在前端的插件安装界面打开本地文件夹或者 zip 文件
- 获取到所有文件，读取 `app.json`，弹出确认创建指定权限 Token 的提示框
- 用户确认后**创建对应的 Token**
- 生成一个随机的 UUID 代表扩展的 ID，这里暂定为 `Extension_UUID`
- 把 `resources` 目录下的静态文件利用后文提到的**静态文件服务上传**到 `https://主控域名/worker-route/static-worker-route/Extension_UUID/` 路由下
- 动态注册前端路由
- 把 `app.json` 和 `Extension_UUID`、Token 等信息**储存到 Kv 内**，namespace 为 `extension-infomation`，key 是 `Extension_UUID`

## 静态文件托管 Worker

为了实现静态文件托管到 Kv，实现了一个简易的 JS 扩展 Worker，名称为 `static-worker`，这个 Worker 会在添加 Server 时预装。

下面是这个 Worker 暴露的接口：

```js
const resourceURL = 'https://WS_HOST/worker-route/static-worker-route/{Extension_UUID}/hello';

// 储存静态资源
fetch(resourceURL,{
    method:'POST',
    body:'你好世界',
    headers:{
        // Bearer + NodeGet Token
        'Authorization':'Bearer Token'
    }
}).then(r => r.text()).then(console.log)

// 获取静态资源（无需 Token）
fetch(resourceURL).then(r => r.text()).then(console.log)
```

因此，在文件上传完成后，其实只是在网页上开一个 `iframe`，类似于：

```html
<iframe src="https://WS_HOST/worker-route/static-worker-route/Extension_UUID/{entry}#?token={}&node=${AGENT_UUID}&theme={theme}"></iframe>
```

## 插件开发及分发

插件开发时应该尽量使用相对路径，比如在 Vite 开发时将 `base` 设为 `./`。

这里提供一个 `demo-extension` 的 Vite 项目。

分发时可以提供打包好的文件的 zip 包。