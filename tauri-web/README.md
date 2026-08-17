# tauri-web

Tauri 桌面应用 + 内嵌 axum Web 服务，通过 rust sqlx 读取 MySQL 数据库 `test.users` 表数据。

- 桌面端：Tauri 2（macOS/Windows/Linux），前端为原生 HTML/CSS/JS，无构建工具
- Web 框架：axum `0.8.9`（内嵌于应用进程，监听 `127.0.0.1:1338`）
- 数据库组件：rust `sqlx`（MySQL 连接池，配置项与连接池参数一一对应）
- 架构：**DDD 分层 + 严格面向接口编程**（domain / application / infra / interfaces / server / providers）
- 功能：users 表**增删改查 + 分页**，前端可视化演示
- 数据读取：提供两种方式
    1. HTTP API：`fetch http://127.0.0.1:1338/api/users`
    2. Tauri 命令：`window.__TAURI__.core.invoke('get_users')`（全量查询演示）

`cargo tauri dev` 运行效果如下：
![tauri-dev.png](tauri-dev.png)

## 分层架构（DDD + 面向接口）

```text
interfaces (handler + routers + commands)  ──►  application (UserService)  ──►  domain (UserRepository 接口)
       ▲                                              ▲                              ▲
       │                                              │                              │
   server （内嵌 axum 启动）                                                              │
       │                                                                                │
   providers（组合根：显式初始化、依赖注入）                                               │
                                                                                        │
       infra（config 配置读取 / persistence 持久化实现 / errors 错误定义）─────────────────┘
```

| 层           | 目录                    | 职责                                                                                       | 依赖          |
|-------------|------------------------|--------------------------------------------------------------------------------------------|-------------|
| domain      | `src/domain`           | 领域实体 `User`、仓储接口 `UserRepository`；不依赖任何框架                                     | `infra::errors`（`RepoError`） |
| application | `src/application`      | 业务编排：`UserService` 只依赖 `UserRepository` 抽象；DTO 入参/出参；用户名校验、分页规则         | domain      |
| infra       | `src/infra`            | 基础设施：`config` 配置读取、`mysql` 连接池；`persistence` sqlx MySQL 实现；`errors` 错误类型    | domain      |
| interfaces  | `src/interfaces`       | 外部交付：`handler`（axum handler）、`routers`（路由装配）、`commands`（Tauri 命令）；不含业务逻辑 | application |
| server      | `src/server`           | 内嵌 axum 启动：监听 `127.0.0.1:port`、绑定路由                                               | interfaces  |
| providers   | `src/providers`        | 组合根：显式初始化（配置 → 连接池 → 仓储 → 服务），以抽象注入具体实现（依赖倒置）                 | 全部          |

设计要点：

- **面向接口**：`domain` 定义 `UserRepository` trait，`infra/persistence/users` 提供 `UserRepoImpl` 实现，
  `application` 通过 `Arc<dyn UserRepository>` 使用抽象；只有 `providers` 知道具体实现是什么。
- **依赖方向**：主要方向为 interfaces → application → domain ← infra；domain 通过 `infra::errors::RepoError`
  返回仓储错误，因此与 infra 存在一处错误类型的依赖交叉。
- **领域层纯净**：`User` 实体不含任何 ORM 注解，数据库行到实体的映射（`sqlx::FromRow`）写在 infra 层。
- **错误分层**：`infra/errors` 定义 `RepoError`（由 sqlx 错误映射）与 `ServiceError`（包装业务语义）；
  `interfaces` 的 `ApiError` 把 `ServiceError` 映射为 HTTP 状态码（400/404/500）。
- **服务列表化**：`application/services/mod.rs` 定义 `Services` 结构体，便于一次性把全部应用服务注册到 Tauri 状态或传递给 server。

## 目录结构

```ini
tauri-web/
├── app.yaml              # 应用配置（端口/日志/redis/mysql 连接池参数，位于项目根目录）
├── public/               # 前端静态资源（Tauri 编译时嵌入）
│   ├── index.html
│   ├── app.js
│   └── style.css
├── sql/
│   └── init.sql          # 建库建表 + 示例数据
└── src-tauri/
    ├── src/
    │   ├── main.rs                   # 桌面端入口（薄层，勿修改）
    │   ├── lib.rs                    # 组合编排：providers 初始化 → 注册状态 → 启动 axum
    │   ├── domain/                   # 领域层：实体 + 仓储接口（零框架依赖）
    │   │   ├── entity/user.rs
    │   │   └── repository/user_repository.rs
    │   ├── application/              # 应用层：业务编排 + DTO
    │   │   ├── dto/
    │   │   │   ├── page.rs           # 分页查询/结果 DTO
    │   │   │   └── user.rs           # 用户新增/更新入参 DTO
    │   │   └── services/
    │   │       └── user_service.rs
    │   ├── infra/                    # 基础设施层：配置读取 + 持久化实现 + 错误类型
    │   │   ├── config/
    │   │   │   ├── app.rs            # AppConfig 加载
    │   │   │   └── mysql.rs          # MySQL 连接池初始化
    │   │   ├── errors/
    │   │   │   └── error.rs          # RepoError / ServiceError
    │   │   └── persistence/
    │   │       └── users/
    │   │           └── user_repo.rs  # UserRepository 的 MySQL 实现
    │   ├── interfaces/               # 接口层：HTTP API 与 Tauri 命令
    │   │   ├── handler/
    │   │   │   ├── user.rs           # axum handler
    │   │   │   └── error.rs          # ApiError 映射
    │   │   ├── routers/
    │   │   │   └── router.rs         # axum 路由装配
    │   │   └── commands/
    │   │       └── user.rs           # Tauri 命令
    │   ├── server/                   # 内嵌 axum 启动
    │   │   └── server.rs
    │   └── providers/                # 组合根：显式初始化 + 依赖注入
    │       └── provider.rs
    ├── capabilities/
    ├── icons/
    ├── build.rs
    └── tauri.conf.json
```

## 环境要求

- Rust 工具链
- Tauri CLI：`cargo install tauri-cli --version ^2.0.0 --locked`
- MySQL 服务（本机 3306）

## 初始化数据库

```shell
# 创建 test 库、users 表并插入示例数据（默认 root 无密码时）
mysql -uroot < sql/init.sql

# 若 root 有密码
mysql -uroot -p < sql/init.sql
```

表结构：

```sql
CREATE TABLE `users`
(
    `id`       bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '自增id',
    `username` varchar(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL DEFAULT '' COMMENT '用户名',
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
```

## 配置

`app.yaml`（项目根目录）中的 `mysql_conf` 为 sqlx 连接池参数：

```yaml
app_name: rs-api
app_port: 1338
monitor_port: 8090       # metrics 指标监控端口（本示例保留字段，未实际使用）
graceful_wait_time: 5    # 优雅退出等待时间（本示例保留字段）
log_level: info

# redis 配置（本示例保留解析，未实际接入）
redis_conf:
  dsn: "redis://:@127.0.0.1:6379/0"
  max_size: 200
  min_idle: 10
  max_lifetime: 1800
  idle_timeout: 300
  connection_timeout: 10

mysql_conf:
  dsn: "mysql://root:root123456@127.0.0.1/test" # dsn连接句柄信息
  max_connections: 100 # 最大连接数
  min_connections: 10  # 最小连接数
  max_lifetime: 1800   # 连接池默认生命周期，单位s
  idle_timeout: 300    # 空闲连接生命周期超时，单位s
  connect_timeout: 10  # 连接超时时间，单位s
```

配置加载优先级：

1. 环境变量 `APP_CONFIG` 指定的路径
2. `./app.yaml`（项目根目录运行）
3. `../app.yaml`（在 src-tauri 目录运行）

## 运行

```shell
cd tauri-web
cargo tauri dev
```

窗口打开后，页面自动分页加载 `users` 表数据，支持：

- **新增**：顶部输入用户名，点击“新增”
- **编辑**：行内“编辑”→ 修改用户名 →“保存”
- **删除**：行内“删除”→ 确认
- **分页**：上一页/下一页、每页条数切换（5/10/20）
- **Tauri 命令查询**：走 `invoke('get_users')` 全量查询演示
- **健康检查**：`GET /api/health`

## API 接口

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
校验：`username` 非空、去首尾空白、长度 ≤ 100（与 `varchar(100)` 一致），不合法返回 `400`；id 不存在返回 `404`。

```shell
# curl 示例
curl "http://127.0.0.1:1338/api/users?page=2&page_size=5"
curl -X POST http://127.0.0.1:1338/api/users -H 'Content-Type: application/json' -d '{"username":"newbie"}'
curl -X PUT  http://127.0.0.1:1338/api/users/1 -H 'Content-Type: application/json' -d '{"username":"heige"}'
curl -X DELETE http://127.0.0.1:1338/api/users/1
```

仅测试 Web API（不起窗口）时，可运行编译产物：

```shell
cd tauri-web
./src-tauri/target/debug/tauri-web
curl http://127.0.0.1:1338/api/health
curl http://127.0.0.1:1338/api/users
```

## 说明与常见问题

- **连接池为什么用 `block_on` 初始化？** Tauri 的 `setup` 钩子运行在主线程、没有 Tokio 上下文，
  而 sqlx 连接池创建需要 Tokio 上下文（否则 panic `this functionality requires a Tokio context`）。
  组合根 `providers::AppProvider::init` 是异步的，`lib.rs` 通过 `tauri::async_runtime::block_on` 提供 Tokio 上下文完成初始化，
  再注册为 Tauri 状态并后台启动 axum server。
- **前端如何使用 `window.__TAURI__`？** 默认 Tauri **不会**把全局 API 注入到 WebView，
  需要在 `tauri.conf.json` 的 `app` 节开启 `"withGlobalTauri": true` 并重新编译，
  否则 `invoke` 会报“Tauri 全局 API 未注入”。本项目的 `get_users` 命令即通过
  `window.__TAURI__.core.invoke('get_users')` 调用。
- **数据库暂不可用？** 连接池使用 lazy 模式（`connect_lazy_with`），启动时不强连数据库，
  应用仍可启动，`/api/health` 会返回 `db: "error"`，数据库恢复后接口自动可用。
- **端口占用**：axum 监听端口由根目录 `app.yaml` 的 `app_port` 控制（默认 1338），
  前端 `public/app.js` 中的 `API_BASE` 需与其保持一致。
- **CORS**：前端页面（dev 为 `http://localhost:1420` 或打包后的 `tauri://localhost`）与 API
  不同源，axum 侧使用 `CorsLayer::permissive()` 放开跨域。
