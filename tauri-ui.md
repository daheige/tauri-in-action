# Tauri 与 Dioxus 关系详解

> 本文档澄清 Tauri 和 Dioxus 两个项目之间的真实关系，纠正"Tauri 的 UI 用到 Dioxus"这一常见误解。

---

## 核心结论

**Tauri 本身并不使用 Dioxus 作为 UI 框架。** 恰恰相反，是 **Dioxus 的桌面端渲染器使用了 Tauri 生态的底层库**（`tao` 和 `wry`）。

两者是**平行协作关系**，共享部分基础设施，但解决完全不同的问题。

---

## 1. 关系图解

```
┌─────────────────────────────────────────────────────────────┐
│                    两个独立项目                              │
│                                                             │
│   Tauri (应用框架)              Dioxus (UI 框架)            │
│   ┌─────────────┐             ┌─────────────────────┐      │
│   │ 前端: 任意   │             │ 前端: Rust (rsx!)   │      │
│   │ (React/Vue/  │             │                     │      │
│   │  Svelte/JS)  │             │ 桌面端渲染:         │      │
│   │             │             │ dioxus-desktop      │      │
│   │ 后端: Rust   │             │    │                │      │
│   │             │             │    ▼                │      │
│   │ 底层依赖:    │◄───────────│  tao (窗口管理)    │      │
│   │  tao + wry  │   共享库    │  wry (WebView)     │      │
│   └─────────────┘             └─────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 具体关系

### 2.1 Dioxus Desktop 依赖 Tauri 的底层库

Dioxus 的桌面端渲染器 `dioxus-desktop` 在底层使用了 **Tauri 团队维护**的两个核心库：

| 库 | 维护方 | Dioxus 用途 | Tauri 用途 |
|----|--------|------------|-----------|
| **tao** | Tauri 团队 | 创建和管理桌面窗口 | 窗口管理 |
| **wry** | Tauri 团队 | 封装系统 WebView 进行渲染 | WebView 渲染 |

Dioxus 官方文档表述为 *"Dioxus desktop 是基于 Tauri 构建的"*，这里的"基于 Tauri"实际上是指基于 `tao` 和 `wry` 这两个底层库，**而不是基于 Tauri 应用框架本身**。

### 2.2 Tauri 的前端可以是任何 Web 技术

Tauri 的前端**不绑定任何 UI 框架**。你可以用：

- React、Vue、Svelte、Angular
- Vanilla JS / TS
- **甚至 Dioxus-Web**（将 Dioxus 编译为 WASM 运行在 Tauri 的 WebView 中）

也就是说，**你可以用 Dioxus 写 Tauri 应用的前端**，但这是一种"组合使用"关系，不是"Tauri 内部依赖 Dioxus"。

### 2.3 两者的根本区别

| 维度 | Tauri | Dioxus Desktop |
|------|-------|---------------|
| **定位** | 应用框架（给前端套桌面壳） | UI 框架（用 Rust 写跨平台 UI） |
| **UI 写法** | HTML/CSS/JS（或前端框架） | Rust 的 `rsx!` 宏 |
| **项目结构** | 前端项目 + Rust 后端项目 | 单一 Rust 项目 |
| **跨语言边界** | 明显（JS ↔ Rust 通过 IPC） | 几乎没有（纯 Rust） |
| **状态管理** | 前端框架负责 + Rust State | Rust Signals |
| **包体积** | 极小（2–10 MB，仅框架运行时） | 较大（含 Rust UI 运行时） |
| **前端生态** | 完整 Web 生态（npm） | 有限（Rust UI 生态） |

> **一句话概括**：Tauri 解决的是"用 Rust 给现有前端应用加一个轻量桌面壳"；Dioxus Desktop 解决的是"我想把整套 UI 和业务状态都留在 Rust 里"。

---

## 3. 为什么会有"Tauri 用到 Dioxus"的误解？

### 3.1 Dioxus 官方文档的表述

Dioxus 说"基于 Tauri 构建"，容易让人误以为两者有上下级或依赖关系。实际上只是共享了 `tao` 和 `wry` 这两个底层库。

### 3.2 两者都出现在 Rust 桌面应用生态中

在 Rust 桌面 GUI 的讨论中，Tauri 和 Dioxus 经常被并列提及，导致概念混淆。社区常将两者作为"Rust 桌面开发方案"的选项对比，进一步加深了"它们是一体的"印象。

### 3.3 可以组合使用

确实可以把 **Dioxus-Web**（编译为 WASM）作为 Tauri 的前端，这种用法让两者看起来像是"一起使用"的关系，但实际上：

- Tauri 的前端换成 React/Vue 也一样工作
- Dioxus 的桌面端换成 `dioxus-native`（WGPU 渲染）就不再需要 `tao`/`wry`

---

## 4. 组合使用场景

虽然 Tauri 不依赖 Dioxus，但以下场景下两者可以**有意组合**：

### 场景 A：Dioxus-Web + Tauri

```
Dioxus 组件 (rsx!) ──► 编译为 WASM ──► 运行在 Tauri WebView 中
                                         └── 通过 Tauri IPC 调用 Rust 命令
```

- 利用 Dioxus 的响应式 UI 能力
- 利用 Tauri 的极小包体积和原生 API 访问
- 适合：需要 Rust 后端 + 轻量部署的桌面应用

### 场景 B：Dioxus Desktop（独立）

```
Dioxus 组件 (rsx!) ──► dioxus-desktop ──► tao + wry ──► 原生窗口 + WebView
```

- 纯 Rust 项目，无需前端构建工具链
- 适合：团队全栈 Rust、不想维护 JS 工具链的项目

---

## 5. 总结

| 问题 | 答案 |
|------|------|
| Tauri 的 UI 会用到 Dioxus 吗？ | **不会**。Tauri 的 UI 是你自己选的任意前端技术 |
| Dioxus 会用到 Tauri 吗？ | **部分会**。Dioxus Desktop 使用了 Tauri 团队维护的 `tao` 和 `wry` 库 |
| 两者是什么关系？ | **平行协作**。共享底层基础设施，解决不同问题 |
| 可以用 Dioxus 写 Tauri 应用吗？ | **可以**。通过 Dioxus-Web 编译为 WASM 作为 Tauri 前端 |
| 应该用 Tauri 还是 Dioxus Desktop？ | 取决于：已有前端代码 → Tauri；纯 Rust 团队 → Dioxus Desktop |

---

## 附录：相关项目归属

| 项目 | 归属 | 用途 |
|------|------|------|
| `tauri` | Tauri 团队 | 应用框架 |
| `tao` | Tauri 团队 | 跨平台窗口管理 |
| `wry` | Tauri 团队 | 跨平台 WebView 封装 |
| `dioxus` | Dioxus 团队 | UI 框架 |
| `dioxus-desktop` | Dioxus 团队 | Dioxus 的桌面渲染器（依赖 tao + wry） |
| `dioxus-web` | Dioxus 团队 | Dioxus 的 Web 渲染器（编译为 WASM） |
| `dioxus-native` | Dioxus 团队 | Dioxus 的原生 GPU 渲染器（实验性，不依赖 tao/wry） |

---

> **文档信息**
> - 整理日期：2026-08-14
> - 来源：Dioxus 官方文档、Tauri 官方文档、社区讨论、源码结构分析
