// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 移动端运行入口 src-tauri
fn main() {
    tauri_demo_lib::run()
}
