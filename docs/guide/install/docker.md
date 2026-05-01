# Docker 安装

## 安装 Server

官方 Docker 镜像由 CI 从源码编译并推送到 GHCR：

- `ghcr.io/nodeseekdev/nodeget-server:latest`：最新发布版本镜像
- `ghcr.io/nodeseekdev/nodeget-server:vX.Y.Z`：Release tag 镜像

### PostgreSQL + NodeGet

```shell
mkdir -p nodeget
touch nodeget/config.toml
curl -fsSL https://raw.githubusercontent.com/NodeSeekDev/NodeGet/main/docker-compose.postgres.yml -o docker-compose.yml
docker compose up -d
```

### SQLite

```shell
mkdir -p nodeget
touch nodeget/config.toml
curl -fsSL https://raw.githubusercontent.com/NodeSeekDev/NodeGet/main/docker-compose.sqlite.yml -o docker-compose.yml
docker compose up -d
```

默认暴露 `3000` 端口。可通过环境变量覆盖常用配置：

```shell
NODEGET_HOST_PORT=2211 \
NODEGET_PORT=2211 \
NODEGET_SERVER_UUID=auto_gen \
NODEGET_LOG_FILTER=info \
docker compose up -d
```

PostgreSQL 部署时如需修改数据库密码，设置 `POSTGRES_PASSWORD` 即可；默认的 `NODEGET_DATABASE_URL` 会自动使用同一组 `POSTGRES_DB` / `POSTGRES_USER` / `POSTGRES_PASSWORD`。

如需使用指定镜像 tag：

```shell
NODEGET_IMAGE=ghcr.io/nodeseekdev/nodeget-server:v0.0.6 docker compose up -d
```

Compose 示例只持久化 `./nodeget/config.toml`。若希望完全手动维护配置文件，可以取消 `NODEGET_CONFIG_FROM_ENV=true`，直接编辑 `./nodeget/config.toml`。

## 安装 Agent

待支持
