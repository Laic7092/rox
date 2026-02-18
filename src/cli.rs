use anyhow::{Context, Result};
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::agent::Agent;
use crate::config::Config;

/// 打印帮助信息
fn print_help() {
    println!("🤖 简易 Rust Agent (Ollama)");
    println!();
    println!("用法：brk <命令>");
    println!();
    println!("命令:");
    println!("  onboard    初始化配置和 workspace");
    println!("  agent      开启交互式对话");
    println!("  help       显示此帮助信息");
    println!();
    println!("示例:");
    println!("  brk onboard              # 初始化配置");
    println!("  brk agent                # 开始对话");
    println!("  brk                      # 显示帮助");
}

/// Onboard 命令 - 初始化配置和 workspace
fn run_onboard() -> Result<()> {
    println!("🚀 初始化 brk 配置...\n");

    let config = Config::default();
    
    // 创建目录
    config.ensure_workspace()
        .context("创建 workspace 目录失败")?;
    config.ensure_sessions()
        .context("创建 sessions 目录失败")?;
    
    println!("✅ 创建目录:");
    println!("   Workspace: {}", config.workspace.root.display());
    println!("   Sessions:  {}", config.session.storage_path.display());
    println!();

    // 创建配置文件
    let config_path = dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".brk")
        .join("config.toml");
    
    config.save(&config_path)
        .context("保存配置文件失败")?;
    println!("✅ 保存配置：{}", config_path.display());
    println!();

    // 创建 AGENT.md
    let agent_path = &config.workspace.agent_file;
    let agent_content = "# 角色定义\n\n你是一个智能助手，旨在帮助用户完成各种任务。\n你具备使用工具的能力，可以协助用户处理文件、获取信息等。\n";
    std::fs::write(agent_path, agent_content)?;
    println!("✅ 创建：{}", agent_path.display());

    // 创建 SOUL.md
    let soul_path = &config.workspace.soul_file;
    let soul_content = "# 对话风格\n\n- 简洁明了\n- 友好专业\n- 用中文回复\n";
    std::fs::write(soul_path, soul_content)?;
    println!("✅ 创建：{}", soul_path.display());

    // 创建 USER.md
    let user_path = &config.workspace.user_file;
    let user_content = "# 用户信息\n\n在此记录你的个人偏好、背景信息和特殊需求。\n\n例如：\n- 偏好的沟通方式\n- 专业领域背景\n- 特定任务需求\n";
    std::fs::write(user_path, user_content)?;
    println!("✅ 创建：{}", user_path.display());
    println!();

    println!("🎉 初始化完成！");
    println!();
    println!("你可以:");
    println!("  1. 编辑 ~/.brk/workspace/*.md 文件自定义你的助手");
    println!("  2. 运行 'brk agent' 开始对话");
    
    Ok(())
}

/// Agent 命令 - 交互式对话
async fn run_agent() -> Result<()> {
    println!("🤖 简易 Rust Agent (Ollama)");
    println!("可用工具：fs_read, fs_write, fs_patch, fs_list, web_search, web_fetch, get_time");
    println!("输入 'quit' 或 'exit' 退出\n");

    // 加载配置
    let config = Config::load_default()?;
    
    // 确保目录存在
    config.ensure_workspace()?;
    config.ensure_sessions()?;
    
    println!("📁 Workspace: {}", config.workspace.root.display());
    println!("📁 Sessions:  {}", config.session.storage_path.display());
    println!("🤖 模型：{}", config.agent.model);
    println!();

    let mut agent = Agent::new(config.agent);
    
    // 使用 BufReader 确保正确处理 UTF-8 多字节字符
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());

    loop {
        print!("👤 你：");
        stdout.flush()?;

        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("👋 再见！");
            break;
        }

        match agent.chat(input).await {
            Ok(reply) => {
                println!("🤖 AI: {}\n", reply);
            }
            Err(e) => {
                println!("❌ 错误：{}\n", e);
            }
        }
    }

    Ok(())
}

/// 主入口函数
pub async fn run_cli() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let command = args[1].to_lowercase();
    
    match command.as_str() {
        "onboard" => run_onboard(),
        "agent" => run_agent().await,
        "help" | "-h" | "--help" | "h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("❌ 未知命令：{}", command);
            eprintln!();
            eprintln!("运行 'brk help' 查看帮助信息");
            std::process::exit(1);
        }
    }
}
