// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    println!("call greet got name: {}", name);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 自定义 tauri funcion command
#[tauri::command]
fn say_hello(name: &str) -> String {
    println!("call say_hello got name: {}", name);
    format!("hello, {}! You're alive!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 移动端运行
    // invoke_handler 用于注册 invoke 相关命令函数
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, say_hello])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
