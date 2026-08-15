# Tauri 2.11.5 架构设计全面分析

> 本文档基于 Tauri 2.11.x 系列（截至 2.11.5）的公开技术资料、官方架构文档、源码结构、Release Notes 及社区实践整理而成，涵盖框架的整体架构、核心子系统、安全模型、渲染层、移动端支持及构建工具链。

---

## 目录

1. [概述](#1-概述)
2. [Workspace 与包结构](#2-workspace-与包结构)
3. [核心运行时架构](#3-核心运行时架构)
4. [底层依赖：Tao 与 Wry](#4-底层依赖tao-与-wry)
5. [IPC 架构：命令系统与事件系统](#5-ipc-架构命令系统与事件系统)
6. [Capability 安全模型](#6-capability-安全模型)
7. [插件系统架构](#7-插件系统架构)
8. [移动端架构（iOS / Android）](#8-移动端架构ios--android)
9. [构建系统与配置架构](#9-构建系统与配置架构)
10. [窗口与 WebView 架构](#10-窗口与-webview-架构)
11. [打包与分发系统](#11-打包与分发系统)
12. [2.11.5 版本特定变更](#12-2115-版本特定变更)
13. [设计哲学与权衡](#13-设计哲学与权衡)
14. [与 Electron 的架构对比](#14-与-electron-的架构对比)

---

## 1. 概述

Tauri 是一个基于 Rust 的跨平台应用开发工具包，用于构建桌面端和移动端应用。其核心设计哲学是**"利用操作系统原生 WebView 渲染前端，用 Rust 编写后端逻辑"**，从而在保证前端开发灵活性的同时，获得极小的包体积、原生性能和内存安全。

### 1.1 核心定位

| 维度 | 设计选择 |
|------|---------|
| **渲染引擎** | 复用 OS 原生 WebView（Windows: WebView2, macOS: WKWebView, Linux: WebKitGTK, iOS: WKWebView, Android: WebView） |
| **后端语言** | Rust（内存安全、零成本抽象、无 GC） |
| **前端技术** | 任意 Web 技术栈（React、Vue、Svelte、Vanilla JS 等） |
| **包体积** | Hello World 约 2–10 MB（对比 Electron 的 80–200 MB） |
| **进程模型** | 单进程多线程（Rust 主进程 + WebView 渲染进程） |
| **跨平台** | Windows、macOS、Linux、iOS、Android |

### 1.2 2.x 系列里程碑

Tauri 2.0 于 2024 年 10 月发布稳定版，是框架的重大演进：

- **移动端支持**：iOS 和 Android 成为一等公民，与桌面端共享同一套前端代码
- **Capability 安全模型**：从 v1 的"默认开放+白名单"反转为"默认拒绝+显式授权"
- **IPC 重写**：支持 Raw Payload，突破 JSON 序列化在大数据传输上的瓶颈
- **插件系统成熟化**：大量原生功能（通知、文件系统、生物识别、深度链接等）迁移到官方版本化插件
- **外部安全审计**：由 Radically Open Security 独立审计，修复了 dev server 暴露、iframe API、scope 验证等安全问题

---

## 2. Workspace 与包结构

Tauri 采用大型 Cargo Workspace 组织，核心 crate 位于 `crates/` 目录下：

```
tauri/
├── crates/
│   ├── tauri/                    # 主入口 crate，聚合所有公共 API
│   ├── tauri-runtime/            # 运行时抽象层（Runtime trait）
│   ├── tauri-runtime-wry/        # Wry 运行时实现
│   ├── tauri-build/              # 构建脚本辅助（build.rs 集成）
│   ├── tauri-codegen/            # 编译期代码生成（配置解析、资源嵌入）
│   ├── tauri-macros/             # 过程宏（#[tauri::command] 等）
│   ├── tauri-utils/              # 通用工具（配置解析、平台检测、CSP、资产管理）
│   ├── tauri-plugin/             # 插件系统基础 crate
│   ├── tauri-bundler/            # 跨平台打包与安装程序生成
│   └── tauri-driver/             # WebDriver 测试客户端
├── packages/
│   ├── api/                      # @tauri-apps/api — JS/TS 前端 API
│   └── cli/                      # @tauri-apps/cli — Node.js CLI 包装器
├── core/                         # 核心运行时源码
└── ARCHITECTURE.md               # 官方架构文档
```

### 关键依赖关系

```
tauri (facade crate)
    ├── tauri-runtime (抽象层)
    │   └── tauri-runtime-wry (Wry 实现)
    │       ├── wry (WebView 封装)
    │       │   ├── tao (窗口管理)
    │       │   └── 平台 WebView API
    │       └── tauri-utils
    ├── tauri-macros
    │   └── tauri-codegen
    ├── tauri-build
    │   └── tauri-codegen
    └── tauri-utils
```

---

## 3. 核心运行时架构

### 3.1 总体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        应用层（开发者代码）                          │
│   ┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│   │  Frontend   │    │  #[tauri::command]│   │  Plugin API    │   │
│   │ (JS/TS/CSS) │    │  Rust Commands    │   │  (Rust/Native) │   │
│   └──────┬──────┘    └─────────────────┘    └─────────────────┘   │
└──────────┼──────────────────────────────────────────────────────────┘
           │ IPC (invoke / emit)
           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Tauri Core Runtime                            │
│   ┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│   │  tauri      │◄──►│  tauri-runtime  │◄──►│  Event System   │   │
│   │  (facade)   │    │  (抽象层)        │    │  (Emitter/Listener)│  │
│   └──────┬──────┘    └────────┬────────┘    └─────────────────┘   │
│          │                    │                                      │
│          │                    ▼                                      │
│          │           ┌─────────────────┐                            │
│          │           │ tauri-runtime-wry│                           │
│          │           │ (Wry 具体实现)   │                           │
│          │           └────────┬────────┘                            │
└──────────┼────────────────────┼──────────────────────────────────────┘
           │                    │
           │                    ▼
           │           ┌─────────────────┐
           │           │      Wry        │
           │           │  (WebView 封装)  │
           │           └────────┬────────┘
           │                    │
           ▼                    ▼
┌─────────────────┐    ┌─────────────────┐
│   Tao (窗口)     │    │  OS Native WebView│
│  (窗口/事件循环) │    │  (WebView2/WKWebView/│
│                 │    │   WebKitGTK/WebView) │
└─────────────────┘    └─────────────────────┘
```

### 3.2 核心 Crate 职责

#### `tauri` — 主入口与 Facade

- 聚合并重导出 `tauri-runtime`、`tauri-macros`、`tauri-utils` 的所有公共 API
- 在编译期读取 `tauri.conf.json`，生成配置结构体并注入到运行时
- 运行时负责：
  - 脚本注入（polyfills、原型修订、CSP）
  - 系统交互 API 的宿主
  - 自动更新管理
  - 事件总线（Emitter/Listener）
- 提供 `Builder` 模式构建应用：`tauri::Builder::default().setup(...).run(...)`

#### `tauri-runtime` — 运行时抽象层

- 定义 `Runtime`、`RuntimeHandle`、`Dispatch` 等核心 trait
- 抽象窗口创建、WebView 管理、事件循环、IPC 处理
- 使 Tauri 可以切换底层实现（当前仅 Wry 实现，但设计上支持多后端）

#### `tauri-runtime-wry` — Wry 运行时实现

- `tauri-runtime` trait 的 Wry 后端实现
- 直接操作系统级交互：打印、显示器检测、窗口管理
- 处理平台特定的 WebView 细节（Windows: webview2-com, macOS: objc2, Linux: webkit2gtk, Android: JNI）

#### `tauri-macros` — 过程宏

- `#[tauri::command]`：将 Rust 函数暴露为前端可调用的 IPC 命令
- `tauri::generate_context!()`：编译期生成应用上下文（配置、资源句柄）
- 内部依赖 `tauri-codegen` 进行代码生成

#### `tauri-codegen` — 编译期代码生成

- 解析 `tauri.conf.json` 并生成 `Config` 结构体
- 嵌入、哈希、压缩静态资源（图标、系统托盘图标、前端构建产物）
- 生成前端资源的路径映射

#### `tauri-utils` — 通用工具

- 配置文件解析（JSON/JSON5/TOML）
- 平台三元组检测（target triple）
- CSP（内容安全策略）注入与管理
- 资产管理与路径处理

---

## 4. 底层依赖：Tao 与 Wry

Tauri 不直接调用操作系统 API，而是通过两个上游 crate 完成底层工作：

### 4.1 Tao — 跨平台窗口管理

Tao 是 Tauri 团队维护的跨平台应用窗口创建库，基于 Winit 的分支并扩展了桌面级功能。

```
Tao 职责：
├── 窗口创建与管理（创建、销毁、最小化、全屏、透明等）
├── 事件循环（RunEvent: Exit, WindowEvent, Ready, Resumed, Suspended 等）
├── 系统托盘（TrayIcon、菜单、点击事件）
├── 全局快捷键（GlobalShortcut）
├── 显示器检测（Monitor、工作区、DPI）
├── 窗口菜单（Menu、MenuItem、预定义菜单项）
├── 拖拽区域（data-tauri-drag-region）
└── 平台特定扩展（macOS: 激活策略、代理；Windows: DWM）
```

**Tao 与 Winit 的区别**：
- Tao 扩展了系统托盘、全局快捷键、菜单等桌面应用必需功能
- 对移动端（iOS/Android）有更好的事件支持（Resumed/Suspended）
- 2.11.x 系列中 Tao 版本约为 0.36.x

### 4.2 Wry — 跨平台 WebView 封装

Wry 是 Tauri 团队维护的跨平台 WebView 渲染库，将各平台的原生 WebView API 统一为 Rust API。

```
Wry 职责：
├── WebView 创建与配置（URL、大小、可见性、DevTools 等）
├── 自定义协议（Custom Protocol）—— 拦截 scheme:// 请求
├── IPC 消息通道（WebView ↔ Rust 的底层桥接）
├── 脚本注入（初始化脚本、用户脚本）
├── Cookie 管理（读取、设置、删除）
├── 导航控制（前进、后退、重载、停止）
├── 缩放与可见性控制
├── 文件拖拽（DragDrop）
└── 平台特定功能（macOS: 链接预览、iOS: input accessory view）
```

**各平台实现**：

| 平台 | 底层 API | 依赖 crate |
|------|---------|-----------|
| Windows | WebView2 (Edge) | `webview2-com`, `windows` |
| macOS | WKWebView | `objc2`, `objc2-app-kit` |
| Linux | WebKitGTK | `webkit2gtk`, `gtk` |
| iOS | WKWebView | `objc2`（通过 Tao 的 AppDelegate） |
| Android | Android WebView | `jni`（JNI 桥接） |

---

## 5. IPC 架构：命令系统与事件系统

Tauri 的 IPC（Inter-Process Communication）是连接前端（WebView 中的 JS）与后端（Rust 进程）的核心桥梁。尽管技术上属于同一 OS 进程内的通信，但命名沿用 IPC 惯例。

### 5.1 架构概览

```
Frontend (JS/TS)                              Backend (Rust)
┌─────────────────┐                          ┌─────────────────┐
│ @tauri-apps/api │                          │  tauri::command │
│   invoke()      │ ──JSON/Raw Payload──►    │  dispatch       │
│   emit()        │ ◄──JSON/Raw Payload──    │  event handler  │
│   listen()      │                          │  State<T>       │
└─────────────────┘                          └─────────────────┘
        │                                            │
        ▼                                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    IPC Transport Layer                        │
│   Windows: chrome.webview.postMessage                       │
│   macOS:   webkit.messageHandlers.ipc.postMessage           │
│   Linux:   Custom URI scheme handler                        │
│   iOS:     WKScriptMessageHandler                           │
│   Android: @JavascriptInterface (JNI)                       │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Command 系统

#### 基本用法

```rust
#[tauri::command]
async fn greet(name: String) -> Result<String, String> {
    Ok(format!("Hello, {}!", name))
}

// 注册到 Builder
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![greet])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

```typescript
import { invoke } from '@tauri-apps/api/core';
const response = await invoke('greet', { name: 'Tauri' });
```

#### 执行流程

```
1. 前端调用 invoke('cmd_name', { arg: 'value' })
2. @tauri-apps/api 将参数序列化为 JSON
3. 通过平台特定的 host-bridge 通道发送消息
   - Windows: chrome.webview.postMessage
   - macOS: webkit.messageHandlers.ipc.postMessage
   - Linux: custom URI scheme
4. tauri-runtime-wry 接收平台消息
5. 路由到 crates/tauri/src/ipc/invoke.rs
6. 构造 Invoke 结构体（命令名 + payload + resolver）
7. Capability ACL 检查：验证窗口是否有权限调用该命令
8. 参数反序列化：serde 将 JSON 映射到 Rust 函数参数
9. 执行 #[tauri::command] 函数
10. 结果序列化为 JSON，通过 resolver 通道返回
11. 前端 Promise resolve/reject
```

#### 2.x 关键改进：Raw Payload

Tauri 2.0 重写了 IPC 层，支持**原始载荷（Raw Payload）**：

- **v1 限制**：所有 IPC 载荷必须经过 JSON 序列化/反序列化，传输数 KB 以上数据时开销明显
- **v2 改进**：支持直接传输原始字节，开发者可使用自定义序列化（BSON、Protobuf、Avro 等）
- **文件读取**：对于直接从文件系统读取到 WebView 的场景，仍推荐使用 `convertFileSrc` 自定义协议（绕过 IPC  entirely）

#### 命令宏特性（2.11 新增）

2.11.0 新增 `rename` 属性，允许命令注册名与函数名不同：

```rust
#[tauri::command(rename = "my-greet")]
fn greet() {}
```

### 5.3 事件系统

Tauri 提供基于发布-订阅模式的事件总线，支持窗口级和全局事件。

#### 核心 API

| API | 说明 |
|-----|------|
| `emit(event, payload)` | 从 Rust 向所有窗口广播事件 |
| `emit_to(target, event, payload)` | 向特定窗口/标签发送事件 |
| `listen(event, handler)` | 在 Rust 侧监听事件 |
| `window.listen(event, handler)` | 在特定窗口监听事件 |
| `window.emit(event, payload)` | 从 JS 向 Rust 发送事件 |

#### 状态管理集成

事件系统与 `State<T>` 管理器深度集成：

```rust
struct AppState {
    db: Database,
}

#[tauri::command]
async fn update_data(state: State<'_, AppState>) -> Result<(), String> {
    state.db.update().await.map_err(|e| e.to_string())
}
```

- `State<T>` 使用 `Arc<T>` 内部实现，支持跨命令共享状态
- 推荐配合 `tokio::sync::RwLock` 或 `parking_lot` 使用

---

## 6. Capability 安全模型

Tauri 2 的安全模型是其架构的核心差异化设计，采用**默认拒绝（Deny-by-Default）**的 Capability-Based Access Control。

### 6.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    Trust Boundary                            │
│                                                             │
│   ┌─────────────────┐         ┌─────────────────────────┐   │
│   │  WebView        │         │  Rust Core / Plugins    │   │
│   │  (Untrusted)    │◄───────►│  (Trusted, Full Access) │   │
│   │  JS/TS 代码      │   IPC   │  文件系统、网络、原生 API │   │
│   └─────────────────┘         └─────────────────────────┘   │
│          │                            │                     │
│          │  Capability 检查            │                     │
│          │  (ACL 拦截未授权调用)        │                     │
│          ▼                            ▼                     │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  capabilities/*.json — 显式权限声明                  │   │
│   │  每个窗口/标签独立配置可访问的命令与资源范围         │   │
│   └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Capability 文件结构

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "description": "主窗口的能力配置",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:allow-read",
    {
      "identifier": "fs:scope",
      "allow": ["$APPDATA/**", "$HOME/Documents/*"]
    },
    "http:allow-request",
    {
      "identifier": "http:scope",
      "allow": ["https://api.example.com/**"]
    },
    "shell:allow-open"
  ]
}
```

### 6.3 与 v1 安全模型的对比

| 维度 | Tauri v1 | Tauri v2 |
|------|---------|---------|
| **默认策略** | 默认开放，通过 allowlist 限制 | 默认拒绝，通过 capabilities 授权 |
| **粒度** | 全局 allowlist | 按窗口/标签独立配置 |
| **范围限制** | 粗粒度（允许/拒绝整个 API） | 细粒度（文件路径、HTTP 端点、命令级别） |
| **插件权限** | 内嵌在 tauri.conf.json | 独立 permissions/*.toml + capabilities/*.json |
| **审计性** | 配置分散 | 清晰的 capabilities/ 目录即授权策略 |

### 6.4 权限层次

```
Permission (权限定义)
    └── 由插件或核心在 permissions/*.toml 中定义
        └── 描述一个 API 操作（如 fs:allow-read）
            └── 可包含默认 scope（允许的路径/URL）

Capability (能力组合)
    └── 在 capabilities/*.json 中定义
        └── 将一组 permissions 授予特定窗口/标签
            └── 可进一步限制 scope（覆盖或缩小默认范围）

ACL Runtime (运行时检查)
    └── 每次 invoke() 前检查
        └── 验证：窗口标识 → 匹配的 capability → permission 存在 → scope 匹配
```

### 6.5 安全最佳实践

- **最小权限原则**：每个窗口只授予其必需的权限
- **Scope 限制**：文件系统操作限定到特定目录，HTTP 请求限定到特定域名
- **分离窗口权限**：主交互窗口权限较宽，托盘图标/通知窗口权限极窄
- **无隐藏权限**：所有权限声明在 `capabilities/` 目录中，安全审计一目了然

---

## 7. 插件系统架构

Tauri 2 的插件系统是其扩展性的核心，将大量原生功能从核心中解耦为独立、版本化的插件。

### 7.1 插件结构

```
tauri-plugin-<name>/
├── src/
│   ├── lib.rs              # 插件入口（Builder、初始化）
│   ├── commands.rs         # #[tauri::command] 定义
│   ├── desktop.rs          # 桌面端实现（Windows/macOS/Linux）
│   ├── mobile.rs           # 移动端桥接（iOS/Android）
│   └── models.rs           # 请求/响应类型定义
├── permissions/
│   └── default.toml        # 插件默认权限定义
├── ios/                    # iOS 原生代码（Swift/ObjC）
│   └── Sources/
│       └── Plugin.swift
├── android/                # Android 原生代码（Kotlin）
│   └── src/main/java/
│       └── Plugin.kt
├── guest-js/               # 前端 JS/TS API
│   └── index.ts
├── build.rs                # 构建脚本（编译移动原生代码）
└── Cargo.toml
```

### 7.2 插件数据流

```
Frontend (JS/TS)
    │
    │ import { pluginAPI } from 'tauri-plugin-<name>'
    ▼
invoke('plugin:<name>|command', args)
    │
    ▼
Tauri Core IPC
    │
    ▼
tauri-plugin-<name> (Rust)
    │
    ├──► Desktop: 直接调用系统 API
    │
    └──► Mobile: run_mobile_plugin() → 平台桥接
            │
            ├──► iOS: Swift Plugin → iOS Framework API
            │
            └──► Android: Kotlin Plugin → Android SDK API
```

### 7.3 移动端桥接机制

#### iOS 桥接

- 使用 `swift-rs` 编译模型将 Swift 代码与 Rust 链接
- 通过 `PluginManager` 单例注册和调度插件
- 部分高级插件使用 `@_cdecl` FFI 直接绕过 `run_mobile_plugin` 以避免 `PluginManager` 重复实例化问题
- Rust 通过 `objc2` crate 与 Objective-C runtime 交互

#### Android 桥接

- 使用 `jni` crate 进行 JNI 调用
- Tauri 生成 Kotlin 插件基类，开发者继承实现
- 通过 `TauriActivity` 和 `TauriPlugin` 基类管理生命周期
- 标准 `run_mobile_plugin` 调度在 Android 上工作稳定

### 7.4 官方插件生态

Tauri 2.x 提供约 20+ 官方插件，涵盖：

| 类别 | 插件 |
|------|------|
| **文件系统** | `fs`（文件读写）、`path`（路径工具） |
| **网络** | `http`（HTTP 请求）、`websocket` |
| **系统** | `os`（系统信息）、`process`（进程管理）、`shell` |
| **UI** | `dialog`（对话框）、`notification`、 `window-state` |
| **存储** | `store`（键值存储）、`sql`（SQLite）、`stronghold`（加密存储） |
| **硬件** | `barcode-scanner`、`biometric`、`nfc` |
| **深度集成** | `deep-link`、`cli`（命令行参数）、`updater` |
| **单实例** | `single-instance` |

---

## 8. 移动端架构（iOS / Android）

Tauri 2 将移动端提升为一等公民，架构上通过**库化编译**和**平台桥接**实现与桌面端共享 Rust 核心代码。

### 8.1 项目结构

```
src-tauri/
├── src/
│   ├── lib.rs          # 移动+桌面共享入口（#[cfg_attr(mobile, tauri::mobile_entry_point)]）
│   └── main.rs         # 桌面专属入口（调用 lib::run()）
├── gen/
│   ├── android/        # 生成的 Android 项目（Gradle + Kotlin）
│   └── apple/          # 生成的 iOS 项目（Xcode + Swift）
├── Cargo.toml
└── tauri.conf.json
```

### 8.2 移动端编译模型

```
桌面端构建：
  cargo build --bin app
    └── 生成可执行二进制文件

移动端构建：
  cargo build --lib
    └── 生成静态库（.a / .so）
        └── 被平台框架加载：
            ├── iOS: Xcode 项目 → Swift UI → 加载 Rust 静态库
            └── Android: Gradle 项目 → Kotlin → 通过 JNI 加载 Rust 库
```

### 8.3 移动端运行时架构

```
┌─────────────────────────────────────────────────────────────┐
│                      iOS 运行时                              │
│                                                             │
│   ┌─────────────┐         ┌─────────────────────────────┐   │
│   │  Swift UI   │         │  Tao AppDelegate            │   │
│   │  (入口层)    │◄───────►│  - 生命周期管理              │   │
│   └──────┬──────┘         │  - 场景(Scene)管理           │   │
│          │                └─────────────────────────────┘   │
│          │                           │                      │
│          ▼                           ▼                      │
│   ┌─────────────┐         ┌─────────────────────────────┐   │
│   │  WKWebView  │◄───────►│  Rust Library (libapp.a)   │   │
│   │  (UI 渲染)   │  IPC    │  - Tauri Runtime            │   │
│   └─────────────┘         │  - Commands / Plugins        │   │
│                           └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                     Android 运行时                           │
│                                                             │
│   ┌─────────────┐         ┌─────────────────────────────┐   │
│   │  Kotlin     │         │  TauriActivity              │   │
│   │  (入口层)    │◄───────►│  - 生命周期管理              │   │
│   └──────┬──────┘         │  - WebView 初始化            │   │
│          │                └─────────────────────────────┘   │
│          │                           │                      │
│          ▼                           ▼                      │
│   ┌─────────────┐         ┌─────────────────────────────┐   │
│   │  WebView    │◄───────►│  Rust Library (libapp.so)  │   │
│   │  (UI 渲染)   │  JNI    │  - Tauri Runtime            │   │
│   └─────────────┘         │  - Commands / Plugins        │   │
│                           └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 8.4 多窗口支持（2.11 新增）

2.11.0 引入移动端多窗口支持：
- **Android**: Activity Embedding — 在同一任务中嵌入多个 Activity
- **iOS**: Scenes API — 利用 iOS 13+ 的 UIScene 支持多窗口/多场景

---

## 9. 构建系统与配置架构

### 9.1 配置文件体系

Tauri 使用分层配置架构：

```
src-tauri/
├── tauri.conf.json              # 主配置（必需）
├── tauri.linux.conf.json        # Linux 覆盖（可选）
├── tauri.macos.conf.json        # macOS 覆盖（可选）
├── tauri.windows.conf.json      # Windows 覆盖（可选）
├── tauri.android.conf.json      # Android 覆盖（可选）
├── tauri.ios.conf.json          # iOS 覆盖（可选）
└── capabilities/
    └── default.json             # 默认能力配置
```

### 9.2 tauri.conf.json 结构

```json
{
  "productName": "My App",
  "version": "1.0.0",
  "identifier": "com.example.myapp",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [{ "title": "My App", "width": 800, "height": 600 }],
    "security": {
      "csp": null,
      "capabilities": ["default"]
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/icon.icns"]
  },
  "plugins": {}
}
```

### 9.3 构建流程

```
tauri dev
    │
    ├──► 1. 执行 beforeDevCommand（启动前端 dev server）
    │
    ├──► 2. 编译 Rust 项目（下载依赖、编译 crate）
    │      - tauri-build 执行 build.rs
    │      - tauri-codegen 解析配置、嵌入资源
    │      - tauri-macros 展开 command 宏
    │
    ├──► 3. 启动应用窗口，加载 devUrl
    │      - 注入初始化脚本（IPC bridge、polyfills）
    │      - 启用 DevTools
    │
    └──► 4. 文件监听（热重载）
           - 前端变更：dev server HMR
           - Rust 变更：重新编译并重启窗口

tauri build
    │
    ├──► 1. 执行 beforeBuildCommand（vite build 等）
    │
    ├──► 2. 编译 Rust 项目（release 模式）
    │      - 前端静态资源嵌入二进制
    │      - 图标压缩、哈希处理
    │      - ACL 优化（removeUnusedCommands）
    │
    ├──► 3. tauri-bundler 生成平台安装包
    │      - Windows: MSI / NSIS
    │      - macOS: DMG / App Bundle
    │      - Linux: DEB / RPM / AppImage
    │      - iOS: IPA
    │      - Android: APK / AAB
    │
    └──► 4. 代码签名（可选）
```

### 9.4 编译期代码生成

`tauri-build` + `tauri-codegen` 在编译期完成以下工作：

1. **配置解析**：读取 `tauri.conf.json`（及平台覆盖文件），生成强类型的 `Config` 结构体
2. **资源嵌入**：将 `frontendDist` 的静态文件、图标、系统托盘图标嵌入为编译期字节数组
3. **资源哈希**：为嵌入资源计算哈希值，用于缓存失效和完整性校验
4. **CSP 生成**：根据配置自动生成 Content-Security-Policy 头
5. **ACL 优化**（2.4+）：根据 `capabilities` 中实际使用的命令，移除未使用的插件命令，减小二进制体积

---

## 10. 窗口与 WebView 架构

### 10.1 窗口模型

Tauri 2 采用 **Window + WebView** 的复合模型：

```
App
├── Window (Tao)
│   ├── 标题栏、边框、尺寸、位置
│   ├── 系统菜单、上下文菜单
│   ├── 拖拽区域（data-tauri-drag-region）
│   └── 事件：聚焦、缩放、移动、关闭
│
└── WebView (Wry)
    ├── 渲染前端 HTML/CSS/JS
    ├── URL：本地文件（WebviewUrl::App）或外部（WebviewUrl::External）
    ├── DevTools、Zoom、Cookie
    └── 与 Window 一对一绑定（不可跨窗口移动）
```

### 10.2 WebviewWindow 与独立 Webview

Tauri 2 支持两种 WebView 创建模式：

| 模式 | API | 说明 |
|------|-----|------|
| **WebviewWindow** | `WebviewWindowBuilder::new()` | 同时创建 Window + WebView（传统模式） |
| **独立 Webview** | `WebviewBuilder::new(&window, ...)` | 在已有 Window 中创建子 WebView（多标签/分栏） |

独立 Webview 的用例：
- 浏览器式多标签界面
- 主窗口内嵌外部网页（如侧边栏加载第三方内容）
- 画中画/浮动预览窗口

### 10.3 窗口间通信

由于 WebView 与 Window 紧密耦合，不支持跨窗口"reparenting"（与 Electron 的 `WebContents` 不同）。跨窗口状态同步通过以下方式：

1. **Rust State**：将共享状态保存在 Rust 侧的 `State<T>` 中，各窗口通过命令读写
2. **事件广播**：使用 `emit()` / `emit_to()` 在窗口间传递消息
3. **状态水合**：新窗口创建时从 Rust 后端获取初始状态

---

## 11. 打包与分发系统

### 11.1 tauri-bundler 架构

`tauri-bundler` 负责将编译后的 Rust 二进制文件与前端资源打包为平台特定的安装程序：

```
tauri build
    │
    └──► tauri-bundler
            │
            ├──► Windows
            │      ├── MSI（Windows Installer）
            │      │   └── WiX Toolset 生成 .msi
            │      ├── NSIS（Nullsoft Scriptable Install System）
            │      │   └── 轻量安装程序，支持自定义脚本
            │      └── 代码签名（signtool / Azure Sign Tool）
            │
            ├──► macOS
            │      ├── DMG（磁盘映像）
            │      ├── App Bundle（.app）
            │      └── 代码签名 + 公证（codesign + notarytool）
            │
            ├──► Linux
            │      ├── DEB（Debian/Ubuntu）
            │      ├── RPM（Fedora/openSUSE）
            │      └── AppImage（通用可执行映像）
            │
            ├──► iOS
            │      └── IPA（通过 Xcode 构建）
            │
            └──► Android
                   └── APK / AAB（通过 Gradle 构建）
```

### 11.2 自动更新（Updater）

Tauri 内置自动更新系统：

- **服务端**：提供 JSON 端点返回最新版本信息（版本号、下载 URL、release notes、签名）
- **客户端**：启动时检查更新，下载差分包或完整包，验证 Ed25519 签名，安装并提示重启
- **配置**：在 `tauri.conf.json > plugins > updater` 中配置公钥和端点

### 11.3 资源与外部二进制

- `resources`：将额外文件打包到应用目录（配置文件、数据库模板等）
- `externalBin`：打包外部可执行文件（辅助工具、CLI 等）
- 均支持平台条件选择（`"resources": { "macos": [...], "windows": [...] }`）

---

## 12. 2.11.5 版本特定变更

Tauri 2.11.5 于 2026 年 7 月 1 日发布，是 2.11 系列的维护性更新：

### 12.1 2.11.x 系列主要变更回顾

#### 2.11.0（2026-04-30）— 功能版本

**新功能**：
- `Bring All to Front` 预定义菜单项类型
- `#[tauri::command]` 宏支持 `rename` 属性，允许命令注册名与函数名不同
- macOS 支持在同一主线程步骤中同时设置图标和图标模板状态，防止闪烁
- `data-tauri-drag-region="deep"` — 点击不可点击的子元素也能触发拖拽
- WebView 选项控制浏览器级自动填充行为（Windows WebView2）
- `eval_with_callback` — 在 Tauri WebView API 和运行时调度层中添加带回调的 JS 执行
- 为 `async_runtime` 启用 `track_caller` 属性，提供更准确的日志和 panic 位置信息
- Android 和 iOS 支持文件关联（File Association）
- Android 触发 `RunEvent::Opened`
- 移动端目标传播 `Event::Suspended` 和 `Event::Resumed`
- Android（Activity Embedding）和 iOS（Scenes）支持创建多窗口
- Linux 添加 `dbus` feature flag（默认启用），用于主题检测
- macOS 和 iOS 添加 Web 内容进程终止处理器

**Bug 修复**：
- 修复将窗口定位到另一显示器时的初始位置问题
- 修复 macOS 显示器工作区 Y 坐标位置
- `on_new_window` 传递的新窗口处理器不再要求 `Sync`，且在 Windows 上运行在主线程，与其他平台对齐

#### 2.11.1–2.11.4（依赖升级）

- 主要升级 `tauri-runtime`、`tauri-utils`、`tauri-runtime-wry` 的依赖版本
- 2.11.4 修复 Windows 上内部获取 DPI 时泄漏 `HDC` 句柄的问题，同时提升无装饰窗口的缩放速度
- 2.11.3 修复 `cookies_for_url` 在与其他窗口/WebView 方法同时调用时可能死锁的问题；修复移动端 `RefCell BorrowMutError` panic（`Resumed`/`Suspended` 事件分支在窗口事件处理器和 `RunEvent` 回调中持有 `windows` 借用，导致创建或关闭窗口时 panic）

### 12.2 2.11.5 具体变更

**发布日期**：2026 年 7 月 1 日

**核心修复**：
> **Fix hotpatching when the bin target's crate name differs from the package name** (PR #5720 by @nicoburns)

- **问题**：当 `Cargo.toml` 中 `[[bin]]` 的 `name` 与 `package.name` 不一致时，某些构建时功能（如资源路径解析）可能无法正确匹配
- **影响**：使用非默认二进制名称（如 workspace 中多 bin target 的项目）可能出现资源加载失败
- **修复**：改进了 crate 名称与 bin target 名称的差异处理逻辑

**版本定位**：2.11.5 是维护性补丁版本，无架构级变更，主要修复边缘场景下的构建稳定性问题。

---

## 13. 设计哲学与权衡

### 13.1 核心设计哲学

| 原则 | 体现 |
|------|------|
| **操作系统优先** | 复用 OS 原生 WebView，不捆绑浏览器引擎，最小化包体积 |
| **安全默认** | Capability 模型默认拒绝所有权限，显式授权才能访问系统资源 |
| **Rust 原生** | 后端完全用 Rust 编写，利用所有权系统保证内存安全，无 GC 开销 |
| **前端自由** | 不绑定特定前端框架，React/Vue/Svelte/Vanilla 均可 |
| **跨平台一致** | 一套代码覆盖桌面+移动端，通过平台覆盖文件处理差异 |
| **模块化扩展** | 核心精简，功能通过插件系统扩展，官方维护版本化插件 |

### 13.2 架构权衡

| 优势 | 代价 |
|------|------|
| 极小包体积（2–10 MB） | 各平台 WebView 行为/特性不一致，需处理兼容性 |
| 内存安全（Rust） | 学习曲线陡峭，团队需掌握 Rust |
| 安全默认的 Capability 模型 | 配置复杂度增加，每个命令都需显式授权 |
| 前端技术栈自由 | 无法利用 Electron 的成熟 Node.js 生态（如原生 Node 模块） |
| 移动端支持 | 移动端体验仍有粗糙边缘（Xcode 集成、插件移动适配） |
| 单进程模型简化通信 | WebView 崩溃可能导致整个应用崩溃（取决于平台实现） |

### 13.3 未来方向

- **Servo 作为替代渲染引擎**：Mozilla 的实验性 Rust 浏览器引擎，可能作为可选 WebView 后端，消除对 OS WebView 的依赖
- **WASI 插件架构**：插件以 `.wasm` 形式分发，由 Tauri 运行时加载，利用 WebAssembly 沙箱实现运行时插件安装
- **移动端成熟度提升**：完善 Xcode/Android Studio 集成，补齐插件移动端支持

---

## 14. 与 Electron 的架构对比

| 维度 | Tauri 2.11 | Electron |
|------|-----------|----------|
| **渲染引擎** | OS 原生 WebView（WebView2/WKWebView/WebKitGTK） | 捆绑 Chromium（完整浏览器） |
| **后端运行时** | Rust（编译为原生二进制） | Node.js + V8（解释执行） |
| **包体积** | 2–10 MB | 80–200 MB |
| **内存占用** | ~50 MB（社区测量） | ~120 MB+（社区测量） |
| **进程模型** | 单进程（Rust + WebView 渲染进程） | 多进程（主进程 + 渲染进程 + GPU 进程等） |
| **IPC 机制** | JSON/Raw Payload over host bridge | JSON / structured-clone over `ipcMain`/`ipcRenderer` |
| **安全模型** | Capability-based ACL（默认拒绝） | 无默认限制（需手动配置 `contextIsolation`、`sandbox`） |
| **移动端** | 支持（iOS/Android） | 不支持（桌面端 only） |
| **前端框架** | 任意（通过 dev server / 静态文件） | 任意（通过 webpack/vite 等打包） |
| **原生 API 访问** | 通过 Rust Commands + 插件 | 通过 Node.js 模块 + Preload 脚本 |
| **自动更新** | 内置 Updater 插件 | 需集成 `electron-updater` |
| **插件生态** | ~120 官方+社区插件 | 10,000+ npm 包 |
| **调试工具** | DevTools（WebView 自带）+ Rust 调试 | DevTools + Node 调试 |
| **代码签名** | 内置支持（各平台工具链） | 需手动配置 |
| **许可证** | MIT / Apache 2.0 | MIT |

### 架构选择建议

**选择 Tauri 当**：
- 需要极小的包体积和内存占用
- 团队熟悉 Rust 或愿意学习
- 应用持有敏感数据（密码、密钥），需要强安全边界
- 需要同时支持桌面和移动端
- 不需要依赖特定的 Node.js 原生模块

**选择 Electron 当**：
- 团队不熟悉 Rust，且时间紧迫
- 依赖大量 Node.js/npm 生态（如特定的原生模块、Electron 专属工具）
- 需要完整的 Chromium 特性（如 WebRTC、复杂的 CSS 特性）
- 应用不需要移动端支持，且对包体积不敏感

---

## 附录：关键术语表

| 术语 | 说明 |
|------|------|
| **Tao** | Tauri 的跨平台窗口管理库（基于 Winit 分支） |
| **Wry** | Tauri 的跨平台 WebView 封装库 |
| **WebView2** | Windows 10+ 内置的 Edge 浏览器控件 |
| **WKWebView** | macOS/iOS 内置的 WebKit 渲染控件 |
| **WebKitGTK** | Linux 平台的 WebKit 浏览器控件 |
| **IPC** | Inter-Process Communication，Tauri 前后端通信机制 |
| **Command** | `#[tauri::command]` 标记的 Rust 函数，暴露给前端调用 |
| **Capability** | Tauri 2 的权限配置单元，定义窗口可访问的 API 范围 |
| **ACL** | Access Control List，Tauri 的访问控制列表系统 |
| **CSP** | Content Security Policy，内容安全策略 |
| **Plugin** | Tauri 的扩展模块，提供额外的系统 API |
| **Bundler** | Tauri 的打包工具，生成平台安装程序 |
| **Updater** | Tauri 的自动更新系统 |
| **State** | Tauri 的依赖注入容器，用于跨命令共享状态 |
| **WASI** | WebAssembly System Interface，Tauri 未来插件架构方向 |
| **Servo** | Mozilla 的实验性 Rust 浏览器引擎 |

---

> **文档信息**
> - 版本：Tauri 2.11.5
> - 整理日期：2026-08-14
> - 来源：Tauri 官方文档、GitHub ARCHITECTURE.md、Release Notes、社区实践、安全审计报告
