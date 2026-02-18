pub fn execute() -> String {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    format!("当前时间：{}", now)
}
