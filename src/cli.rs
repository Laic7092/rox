use anyhow::{Context, Result};
use std::path::Path;

use reedline::{Reedline, Signal, DefaultHinter, DefaultCompleter, DefaultPrompt};

use crate::agent::{Agent, SessionManager};
use crate::config::Config;

/// 打印帮助信息
fn print_help() {
    println!("🤖 brk - 本地 AI 助手");
    println!();
    println!("用法：brk <命令>");
    println!();
    println!("命令:");
    println!("  agent           进入交互模式（默认）");
    println!("  session         会话管理");
    println!("  onboard         初始化配置");
    println!("  help            显示此帮助信息");
    println!();
    println!("交互模式命令:");
    println!("  /clear  - 清空当前会话历史");
    println!("  /new [名] - 创建新会话");
    println!("  /quit   - 退出");
    println!();
    println!("Session 子命令:");
    println!("  session list        - 列出所有会话");
    println!("  session delete <ID> - 删除会话");
    println!();
    println!("示例:");
    println!("  brk                 # 开始对话");
    println!("  brk session list    # 查看会话列表");
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

/// Session 命令 - 会话管理
fn run_session() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("❌ 请指定 session 子命令");
        eprintln!();
        eprintln!("用法：brk session <子命令> [参数]");
        eprintln!();
        eprintln!("子命令:");
        eprintln!("  list         列出所有会话");
        eprintln!("  delete <ID>  删除会话");
        std::process::exit(1);
    }

    let config = Config::load_default()?;
    config.ensure_workspace()?;
    config.ensure_sessions()?;

    let subcommand = args[2].to_lowercase();

    match subcommand.as_str() {
        "list" | "l" => session_list(config),
        "delete" | "rm" => {
            if args.len() < 4 {
                eprintln!("❌ 请指定会话 ID");
                eprintln!("用法：brk session delete <ID>");
                std::process::exit(1);
            }
            session_delete(config, &args[3])
        }
        _ => {
            eprintln!("❌ 未知子命令：{}", subcommand);
            eprintln!("运行 'brk session' 查看可用子命令");
            std::process::exit(1);
        }
    }
}

fn session_list(config: Config) -> Result<()> {
    let mut manager = SessionManager::new(config.session.storage_path);
    let _ = manager.load_all();

    let sessions = manager.list();

    if sessions.is_empty() {
        println!("📭 暂无会话");
        return Ok(());
    }

    println!("📋 会话列表:");
    println!();

    for (id, metadata) in sessions {
        let name_str = metadata.name.as_deref().unwrap_or("(未命名)");
        let created = metadata.created_at.format("%Y-%m-%d %H:%M");
        let msgs = metadata.message_count;

        // 只显示 ID 前 8 位
        let short_id = if id.len() > 8 { &id[..8] } else { id };
        println!("{} - {}", short_id, name_str);
        println!("   创建时间：{} | 消息数：{}", created, msgs);
        println!();
    }

    Ok(())
}

fn session_delete(config: Config, id: &str) -> Result<()> {
    let storage_path = config.session.storage_path.clone();
    let mut manager = SessionManager::new(storage_path.clone());
    let _ = manager.load_all();

    // 支持短 ID 匹配
    let matched_id = if manager.get(id).is_some() {
        id.to_string()
    } else {
        // 尝试查找匹配前缀的会话
        let mut found: Option<String> = None;
        for (session_id, _) in manager.list() {
            if session_id.starts_with(id) {
                found = Some(session_id.to_string());
                break;
            }
        }
        match found {
            Some(fid) => fid,
            None => {
                eprintln!("❌ 会话不存在：{}", id);
                std::process::exit(1);
            }
        }
    };

    // 从磁盘删除
    let path = storage_path.join(format!("{}.json", matched_id));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    // 从内存删除
    manager.delete(&matched_id);

    println!("✅ 已删除会话：{}", matched_id);

    Ok(())
}

/// Agent 命令 - 交互式对话
async fn run_agent() -> Result<()> {
    println!("🤖 简易 Rust Agent (Ollama)");
    println!("可用工具：fs_read, fs_write, fs_patch, fs_list, web_search, web_fetch, get_time");
    println!("输入 'quit' 或 'exit' 退出，输入 'help' 查看帮助\n");

    // 加载配置
    let config = Config::load_default()?;

    // 确保目录存在
    config.ensure_workspace()?;
    config.ensure_sessions()?;

    println!("📁 Workspace: {}", config.workspace.root.display());
    println!("📁 Sessions:  {}", config.session.storage_path.display());
    println!("🤖 模型：{}", config.agent.model);
    println!();

    let agent_config = config.agent.clone();
    let mut agent = Agent::new(agent_config.clone(), config.session, config.workspace.root);

    // 显示当前会话信息
    if let Some(session_id) = agent.current_session_id() {
        println!("📝 当前会话：{}", session_id);
    }
    println!();

    // 使用 reedline 处理输入，支持 UTF-8 和行编辑
    let completer = DefaultCompleter::default();
    let hinter = DefaultHinter::default();
    let prompt = DefaultPrompt::default();

    let mut line_editor = Reedline::create()
        .with_hinter(Box::new(hinter))
        .with_completer(Box::new(completer));

    loop {
        let sig = line_editor.read_line(&prompt)?;

        match sig {
            Signal::Success(buffer) => {
                let input = buffer.trim();

                if input.is_empty() {
                    continue;
                }

                // 斜杠命令
                if input.starts_with('/') {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    let cmd = parts.get(0).map(|s| s.to_lowercase()).unwrap_or_default();

                    match cmd.as_str() {
                        "/quit" | "/exit" => {
                            println!("👋 再见！");
                            break;
                        }
                        "/clear" => {
                            agent.clear_history();
                            println!("✅ 已清空当前会话历史\n");
                        }
                        "/new" => {
                            let name = parts.get(1).map(|s| s.to_string());
                            let sm = agent.session_manager_mut();
                            sm.create(name, agent_config.clone());
                            let id = sm.current_session_id().unwrap_or("unknown");
                            println!("✅ 已创建新会话：{}\n", id);
                        }
                        "/help" | "/h" => {
                            println!("命令:");
                            println!("  /clear  - 清空当前会话历史");
                            println!("  /new [名] - 创建新会话");
                            println!("  /quit   - 退出");
                            println!();
                        }
                        _ => {
                            println!("❌ 未知命令：{}", input);
                            println!("输入 /help 查看帮助\n");
                        }
                    }
                    continue;
                }

                // 普通输入命令（兼容旧版）
                if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
                    println!("👋 再见！");
                    break;
                }

                if input.eq_ignore_ascii_case("clear") {
                    agent.clear_history();
                    println!("✅ 已清空当前会话历史\n");
                    continue;
                }

                if input.eq_ignore_ascii_case("help") {
                    println!("命令:");
                    println!("  /clear  - 清空当前会话历史");
                    println!("  /new [名] - 创建新会话");
                    println!("  /quit   - 退出");
                    println!();
                    continue;
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
            Signal::CtrlD => {
                println!("\n👋 再见！");
                break;
            }
            Signal::CtrlC => {
                println!("\n输入 /quit 退出，或继续输入问题");
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
        "agent" | "a" => run_agent().await,
        "session" | "s" => run_session(),
        "onboard" => run_onboard(),
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
