# tauri-web

Tauri 桌面应用 + 内嵌 axum Web 服务，通过 rust sqlx 读取 MySQL 数据库 `test.users` 表数据。

- 桌面端：Tauri 2（macOS/Windows/Linux），前端为原生 HTML/CSS/JS，无构建工具
- Web 框架：axum `0.8.9`（内嵌于应用进程，监听 `127.0.0.1:1338`）
- 数据库组件：rust `sqlx`（MySQL 连接池，配置项与连接池参数一一对应）
- 功能：users 表**增删改查 + 分页**，前端可视化演示
- 数据读取：提供两种方式
  1. HTTP API：`fetch http://127.0.0.1:1338/api/users`
  2. Tauri 命令：`window.__TAURI__.core.invoke('get_users')`（全量查询演示）

## 目录结构

```ini
tauri-web/
├── config/
│   └── app.yaml          # 应用配置（端口/日志/redis/mysql 连接池参数）
├── public/               # 前端静态资源（Tauri 编译时嵌入）
│   ├── index.html
│   ├── app.js
│   └── style.css
├── sql/
│   └── init.sql          # 建库建表 + 示例数据
└── src-tauri/
    ├── src/
    │   ├── main.rs       # 桌面端入口（薄层，勿修改）
    │   ├── lib.rs        # 启动逻辑：加载配置 → 初始化连接池 → 启动 axum
    │   ├── config.rs     # 配置文件解析（yaml）
    │   ├── db.rs         # sqlx MySQL 连接池初始化
    │   ├── models.rs     # users 表结构（FromRow + Serialize）
    │   └── web.rs        # axum 路由 / 处理器
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
CREATE TABLE `users` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '自增id',
    `username` varchar(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL DEFAULT '' COMMENT '用户名',
    PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
```

## 配置

`config/app.yaml` 中的 `mysql_conf` 为 sqlx 连接池参数：

```yaml
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
2. `./config/app.yaml`（项目根目录运行）
3. `../config/app.yaml`（在 src-tauri 目录运行）

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

| 方法 | 路径 | 说明 | 成功响应 |
|---|---|---|---|
| GET | `/api/health` | 健康检查（含 DB 连通性） | `{"status":"ok","db":"ok"}` |
| GET | `/api/users?page=1&page_size=10` | 分页查询 users | `{"total":N,"page":1,"page_size":10,"items":[{id,username}]}` |
| GET | `/api/users/{id}` | 按 id 查询 | `{"id":1,"username":"daheige"}` |
| POST | `/api/users` | 新增用户，body `{"username":"xxx"}` | `201` + 新用户对象 |
| PUT | `/api/users/{id}` | 更新用户名，body `{"username":"xxx"}` | 更新后的用户对象 |
| DELETE | `/api/users/{id}` | 删除用户 | `204 No Content` |

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

- **为什么连接池在异步任务里初始化？** Tauri 的 `setup` 钩子运行在主线程、没有 Tokio 上下文，
  而 sqlx 连接池创建需要 Tokio 上下文（否则 panic `this functionality requires a Tokio context`），
  因此在 `tauri::async_runtime::spawn` 的异步任务中初始化，并通过 `AppHandle::manage` 注册为 Tauri 状态。
- **前端如何使用 `window.__TAURI__`？** 默认 Tauri **不会**把全局 API 注入到 WebView，
  需要在 `tauri.conf.json` 的 `app` 节开启 `"withGlobalTauri": true` 并重新编译，
  否则 `invoke` 会报“Tauri 全局 API 未注入”。本项目的 `get_users` 命令即通过
  `window.__TAURI__.core.invoke('get_users')` 调用。
- **数据库暂不可用？** 连接池使用 lazy 模式（`connect_lazy_with`），启动时不强连数据库，
  应用仍可启动，`/api/health` 会返回 `db: "error"`，数据库恢复后接口自动可用。
- **端口占用**：axum 监听端口由 `config/app.yaml` 的 `app_port` 控制（默认 1338），
  前端 `public/app.js` 中的 `API_BASE` 需与其保持一致。
- **CORS**：前端页面（dev 为 `http://localhost:1420` 或打包后的 `tauri://localhost`）与 API
  不同源，axum 侧使用 `CorsLayer::permissive()` 放开跨域。
