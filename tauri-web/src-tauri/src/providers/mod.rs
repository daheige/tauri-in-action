//! 组合根（Composition Root）：依赖的显式初始化与装配。
//!
//! 按依赖倒置原则，所有具体实现在这里创建，并以其抽象类型
//! （接口）注入到上层；本模块是唯一知道“具体用了什么实现”的地方。
mod provider;

pub use provider::AppProvider;
