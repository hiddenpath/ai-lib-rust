//! 内存基准测试专用示例
//! 用于观察 Rust 版本的极低内存占用

use ai_lib_rust::protocol::ProtocolLoader;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ai-lib-rust 内存压力测试开始 ===");
    println!("当前进程 PID: {}", std::process::id());

    // 1. 初始状态
    println!("状态: 已启动，等待 5 秒以观察基准内存...");
    sleep(Duration::from_secs(5)).await;

    // 2. 加载协议
    let loader = ProtocolLoader::new();
    println!("状态: 正在加载协议...");
    let start = std::time::Instant::now();
    for _ in 0..10 {
        // 使用通用的 load_provider 接口，它会自动尝试本地和 GitHub 路径
        let _ = loader.load_provider("openai").await;
    }
    println!("完成 10 次协议加载，耗时: {:?}", start.elapsed());

    // 3. 保持运行以便观察
    println!("状态: 运行中，请检查内存占用（预计 < 5MB）...");
    println!("按 Ctrl+C 退出");

    loop {
        sleep(Duration::from_secs(10)).await;
    }
}
