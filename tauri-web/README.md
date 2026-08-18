# tauri-web

Tauri 桌面应用 + 内嵌 axum Web 服务，通过 rust sqlx 读取 MySQL 数据库 `test.users` 表数据。

![tauri-dev.png](tauri-dev.png)

## 核心特性

- **Tauri 2 桌面应用**：原生 HTML/CSS/JS 前端，无前端构建工具，打包体积小
- **内嵌 axum Web 服务**：应用进程内启动 axum `0.8.9`，监听 `127.0.0.1:1338`
- **MySQL 数据访问**：基于 rust `sqlx` 连接池，对 `test.users` 表实现**增删改查 + 分页**
- **双交付方式**：
  - HTTP API：`fetch http://127.0.0.1:1338/api/users`
  - Tauri 命令：`window.__TAURI__.core.invoke('get_users')`（全量查询演示）
- **DDD 分层 + 严格面向接口编程**：domain / application / infra / interfaces / server / providers
- **配置驱动**：根目录 `app.yaml` 管理端口、日志、连接池等参数

## 技术栈

| 层级/功能     | 技术                                  | 说明                                |
|--------------|--------------------------------------|-------------------------------------|
| 桌面框架      | Tauri 2                              | 跨平台桌面应用（Windows/macOS/Linux） |
| Web 框架      | axum `0.8.9`                         | 内嵌 HTTP 服务                      |
| 数据库        | MySQL + sqlx `0.8`                   | 异步 SQL 工具包，带连接池            |
| 异步运行时    | Tokio                                | Rust 异步运行时                      |
| 序列化        | serde / serde_json                   | 请求/响应序列化                      |
| 配置解析      | serde_yaml                           | `app.yaml` 配置加载                  |
| 日志          | tracing / tracing-subscriber         | 结构化日志与日志级别控制              |
| 错误处理      | thiserror / anyhow                   | 类型化错误与上下文传播                |

## 架构设计

项目采用 **DDD 分层 + 依赖倒置** 的六边形/整洁架构思想，依赖方向由外层指向内层，基础设施通过接口注入到应用层。

```text
interfaces (handler + routers + commands)
       │
       ▼
    server                    ── 内嵌 axum 启动
       │
       ▼
applications (UserService)
       │
       ▼
   domain (UserRepository trait + RepoError)
       ▲
       │
   infra (config / persistence)
       ▲
       │
  providers (Composition Root)
```

## 分层架构

| 层           | 目录                              | 职责                                                                                       | 依赖          |
|-------------|-----------------------------------|--------------------------------------------------------------------------------------------|--------------|
| domain      | `src/domain`                      | 领域实体 `User`、仓储接口 `UserRepository`、仓储错误 `RepoError`；零框架依赖                 | 无           |
| application | `src/application`                 | 业务编排：`UserService` 依赖 `UserRepository` 抽象；DTO；`ServiceError` 业务错误            | domain       |
| infra       | `src/infra`                       | 基础设施：`config` 配置读取、`mysql` 连接池、`persistence/users` 仓储实现                   | domain       |
| interfaces  | `src/interfaces`                  | 外部交付：`handler`（axum handler）、`routers`（路由装配）、`commands`（Tauri 命令）        | application  |
| server      | `src/server`                      | 内嵌 axum 启动：监听 `127.0.0.1:port`、绑定路由                                            | interfaces   |
| providers   | `src/providers`                   | 组合根：显式初始化（配置 → 连接池 → 仓储 → 服务），依赖倒置注入                             | 全部         |

## 设计要点

- **面向接口编程**：`domain/repository` 定义 `UserRepository` trait，`infra/persistence/users` 提供 `UserRepoImpl` 实现；`application` 通过 `Arc<dyn UserRepository>` 使用抽象，只有 `providers` 知道具体实现。
- **依赖方向单向**：`interfaces → application → domain ← infra`，`domain` 不依赖任何框架或基础设施。
- **领域层纯净**：`User` 实体不含 ORM/框架注解，数据库行到实体的映射（`sqlx::FromRow`）写在 infra 层。
- **错误分层**：
  - `domain/repository/error.rs`：`RepoError`（仓储/数据库错误）
  - `application/services/error.rs`：`ServiceError`（业务校验/不存在/数据访问失败）
  - `interfaces/handler/error.rs`：`ApiError`（HTTP 状态码映射：400/404/500）
- **服务列表化**：`application/services/mod.rs` 定义 `Services` 结构体，便于一次性注册到 Tauri 状态或传递给 `server`。
- **懒连接池**：`infra/config/mysql.rs` 使用 `connect_lazy_with`，启动时不强制连库，数据库恢复后接口自动可用。

## 目录结构

```ini
tauri-web/
├── app.yaml              # 应用配置（端口/日志/redis/mysql 连接池参数）
├── public/               # 前端静态资源（Tauri 编译时嵌入）
│   ├── index.html
│   ├── app.js            # 前端交互：分页/新增/编辑/删除/Tauri 命令
│   └── style.css
├── sql/
│   └── init.sql          # 建库建表 + 示例数据
└── src-tauri/
    ├── src/
    │   ├── main.rs                   # 桌面端入口（薄层）
    │   ├── lib.rs                    # 组合编排：providers 初始化 → 注册状态 → 启动 axum
    │   ├── domain/                   # 领域层
    │   │   ├── entity/
    │   │   │   └── user.rs           # User 实体
    │   │   └── repository/
    │   │       ├── error.rs          # RepoError
    │   │       └── user_repository.rs # UserRepository trait
    │   ├── application/              # 应用层
    │   │   ├── dto/
    │   │   │   ├── page.rs           # 分页 DTO
    │   │   │   └── user.rs           # 用户入参 DTO
    │   │   └── services/
    │   │       ├── error.rs          # ServiceError
    │   │       ├── mod.rs            # Services 列表
    │   │       └── user_service.rs   # 业务编排
    │   ├── infra/                    # 基础设施层
    │   │   ├── config/
    │   │   │   ├── app.rs            # AppConfig 加载
    │   │   │   └── mysql.rs          # MySQL 连接池
    │   │   └── persistence/
    │   │       └── users/
    │   │           ├── mod.rs
    │   │           └── user_repo.rs  # UserRepository 的 MySQL 实现
    │   ├── interfaces/               # 接口层
    │   │   ├── handler/
    │   │   │   ├── error.rs          # ApiError
    │   │   │   ├── mod.rs
    │   │   │   └── user.rs           # axum handler
    │   │   ├── routers/
    │   │   │   ├── mod.rs
    │   │   │   └── router.rs         # axum 路由装配
    │   │   └── commands/
    │   │       ├── mod.rs
    │   │       └── user.rs           # Tauri 命令
    │   ├── server/                   # 内嵌 axum 启动
    │   │   ├── mod.rs
    │   │   └── server.rs
    │   └── providers/                # 组合根
    │       ├── mod.rs
    │       └── provider.rs
    ├── capabilities/
    │   └── default.json              # Tauri 能力配置
    ├── icons/
    ├── build.rs
    └── tauri.conf.json
```

## 环境要求

- Rust 工具链（建议最新 stable）
- Tauri CLI：`cargo install tauri-cli --version ^2.0.0 --locked`
- MySQL 服务（默认连接本机 3306）
-（可选）`mysql` 客户端，用于执行初始化脚本

## 快速开始

### 1. 初始化数据库

```shell
cd tauri-web

# 默认 root 无密码
mysql -uroot < sql/init.sql

# 或 root 有密码
mysql -uroot -p < sql/init.sql
```

`sql/init.sql` 会创建 `test` 库、`users` 表，并插入 12 条示例数据：

```sql
CREATE TABLE IF NOT EXISTS `users` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '自增id',
    `username` varchar(100) NOT NULL DEFAULT '' COMMENT '用户名',
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### 2. 配置

编辑项目根目录 `app.yaml`：

```yaml
app_name: rs-api
app_port: 1338
log_level: info

mysql_conf:
  dsn: "mysql://root:root123456@127.0.0.1/test"
  max_connections: 100
  min_connections: 10
  max_lifetime: 1800
  idle_timeout: 300
  connect_timeout: 10
```

配置加载优先级：

1. 环境变量 `APP_CONFIG` 指定的路径
2. `./app.yaml`（项目根目录运行）
3. `../app.yaml`（在 `src-tauri` 目录运行）

### 3. 运行

```shell
cd tauri-web
cargo tauri dev
```

窗口打开后：

- 自动分页加载 `users` 表数据
- 支持新增、行内编辑、删除、分页、每页条数切换
- 点击“Tauri 命令查询”可演示 `invoke('get_users')` 全量查询
- 点击“健康检查”查看 `/api/health`

## API 文档

Base URL：`http://127.0.0.1:1338`

| 方法     | 路径                               | 说明                              | 成功响应                                                          |
|--------|----------------------------------|---------------------------------|---------------------------------------------------------------|
| GET    | `/api/health`                    | 健康检查（含 DB 连通性）                  | `{"status":"ok","db":"ok"}`                                   |
| GET    | `/api/users?page=1&page_size=10` | 分页查询 users                      | `{"total":N,"page":1,"page_size":10,"items":[{id,username}]}` |
| GET    | `/api/users/{id}`                | 按 id 查询                         | `{"id":1,"username":"daheige"}`                               |
| POST   | `/api/users`                     | 新增用户，body `{"username":"xxx"}`  | `201` + 新用户对象                                                 |
| PUT    | `/api/users/{id}`                | 更新用户名，body `{"username":"xxx"}` | 更新后的用户对象                                                      |
| DELETE | `/api/users/{id}`                | 删除用户                            | `204 No Content`                                              |

分页参数（均可省略）：`page` 默认 1，`page_size` 默认 10、最大 100。

校验规则：`username` 非空、去首尾空白、长度 ≤ 100（与 `varchar(100)` 一致）；不合法返回 `400`，id 不存在返回 `404`。

```shell
# curl 示例
curl "http://127.0.0.1:1338/api/users?page=2&page_size=5"
curl -X POST http://127.0.0.1:1338/api/users -H 'Content-Type: application/json' -d '{"username":"newbie"}'
curl -X PUT  http://127.0.0.1:1338/api/users/1 -H 'Content-Type: application/json' -d '{"username":"heige"}'
curl -X DELETE http://127.0.0.1:1338/api/users/1
```

## 仅测试 Web API（不启动桌面窗口）

```shell
cd tauri-web
cargo build
./src-tauri/target/debug/tauri-web

curl http://127.0.0.1:1338/api/health
curl http://127.0.0.1:1338/api/users
```

## 常见问题

- **连接池为什么用 `block_on` 初始化？**  
  Tauri 的 `setup` 钩子运行在主线程、没有 Tokio 上下文，而 sqlx 连接池创建需要 Tokio 上下文。`providers::AppProvider::init` 是异步的，`lib.rs` 通过 `tauri::async_runtime::block_on` 提供 Tokio 上下文完成初始化，再注册为 Tauri 状态并后台启动 axum server。

- **前端如何使用 `window.__TAURI__`？**  
  默认 Tauri **不会**把全局 API 注入到 WebView，需要在 `tauri.conf.json` 的 `app` 节开启 `"withGlobalTauri": true` 并重新编译，否则 `invoke` 会报“Tauri 全局 API 未注入”。

- **数据库暂不可用怎么办？**  
  连接池使用 lazy 模式（`connect_lazy_with`），启动时不强连数据库，应用仍可启动；`/api/health` 会返回 `db: "error"`，数据库恢复后接口自动可用。

- **端口占用**  
  axum 监听端口由 `app.yaml` 的 `app_port` 控制（默认 1338），前端 `public/app.js` 中的 `API_BASE` 需与其保持一致。

- **跨域**  
  前端页面（dev 为 `http://localhost:1420`，打包后为 `tauri://localhost`）与 API 不同源，axum 侧使用 `CorsLayer::permissive()` 放开跨域。
