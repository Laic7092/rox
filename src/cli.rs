use anyhow::{Context, Result};
use std::path::Path;

use reedline::{Reedline, Signal, DefaultHinter, DefaultCompleter, DefaultPrompt};

use crate::agent::{Agent, LlmClient, SessionManager};
use crate::config::Config;
use crate::tools::ToolExecutor;

/// 打印帮助信息
fn print_help() {
    println!("🤖 rox - 本地 AI 助手");
    println!();
    println!("用法：rox <命令>");
    println!();
    println!("命令:");
    println!("  agent           进入交互模式（默认）");
    println!("  onboard         初始化配置");
    println!("  help            显示此帮助信息");
    println!();
    println!("交互模式命令:");
    println!("  /clear        - 清空当前会话历史");
    println!("  /resume [ID]  - 切换会话（不带参数显示列表）");
    println!("  /quit         - 退出");
    println!("  /help         - 显示帮助");
    println!();
    println!("选项:");
    println!("  rox agent --log     详细日志模式（显示工具调用详情）");
    println!();
}

/// Onboard 命令 - 初始化配置和 workspace
fn run_onboard() -> Result<()> {
    println!("🚀 初始化 rox 配置...\n");

    let config = Config::default();

    config.ensure_workspace()
        .context("创建 workspace 目录失败")?;
    config.ensure_sessions()
        .context("创建 sessions 目录失败")?;

    println!("✅ 创建目录:");
    println!("   Workspace: {}", config.workspace.root.display());
    println!("   Sessions:  {}", config.session.storage_path.display());
    println!();

    let config_path = dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".rox")
        .join("config.toml");

    config.save(&config_path)
        .context("保存配置文件失败")?;
    println!("✅ 保存配置：{}", config_path.display());
    println!();

    let agent_path = &config.workspace.agent_file;
    let agent_content = "# 角色定义\n\n你是一个智能助手，旨在帮助用户完成各种任务。\n你具备使用工具的能力，可以协助用户处理文件、获取信息等。\n";
    std::fs::write(agent_path, agent_content)?;
    println!("✅ 创建：{}", agent_path.display());

    let soul_path = &config.workspace.soul_file;
    let soul_content = "# 对话风格\n\n- 简洁明了\n- 友好专业\n- 用中文回复\n";
    std::fs::write(soul_path, soul_content)?;
    println!("✅ 创建：{}", soul_path.display());

    let user_path = &config.workspace.user_file;
    let user_content = "# 用户信息\n\n在此记录你的个人偏好、背景信息和特殊需求。\n\n例如：\n- 偏好的沟通方式\n- 专业领域背景\n- 特定任务需求\n";
    std::fs::write(user_path, user_content)?;
    println!("✅ 创建：{}", user_path.display());
    println!();

    println!("🎉 初始化完成！");
    println!();
    println!("你可以:");
    println!("  1. 编辑 ~/.rox/workspace/*.md 文件自定义你的助手");
    println!("  2. 运行 'rox agent' 开始对话");

    Ok(())
}

/// 截断字符串（正确处理 UTF-8）
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if i + c.len_utf8() > max_len - 3 {
            result.push_str("...");
            break;
        }
        result.push(c);
    }
    result
}

/// 打印历史消息（限制条数）
fn print_session_history(session_manager: &SessionManager, max_display: usize) {
    if let Some(session) = session_manager.current() {
        let messages = session.context().raw_messages();
        if messages.is_empty() {
            return;
        }

        let display_count = messages.len().min(max_display);
        let skip_count = messages.len().saturating_sub(display_count);

        if skip_count > 0 {
            println!("📜 历史消息：{} 条（显示最近 {} 条）", messages.len(), display_count);
        } else {
            println!("📜 历史消息：{} 条", messages.len());
        }
        println!();

        for msg in messages.iter().skip(skip_count) {
            let preview = truncate_str(&msg.content, 100);
            match msg.role.as_str() {
                "user" => println!("👤 你：{}", preview),
                "assistant" => println!("🤖 AI: {}", preview),
                "tool" => println!("🔧 工具：{}", preview),
                _ => {}
            }
        }
        println!();
    }
}

/// 打印交互帮助
fn print_interactive_help() {
    println!("可用命令:");
    println!("  /clear        - 清空当前会话历史");
    println!("  /resume [ID]  - 切换会话（不带参数显示列表）");
    println!("  /quit         - 退出");
    println!("  /help         - 显示此帮助");
    println!();
}

/// 处理斜杠命令，返回是否退出
fn handle_command(session_manager: &mut SessionManager, cmd: &str) -> bool {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let command = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match command.as_str() {
        "/quit" | "/exit" => {
            println!("👋 再见！");
            true
        }
        "/clear" => {
            if let Some(session) = session_manager.current_mut() {
                session.context_mut().clear();
            }
            let _ = session_manager.save_current();
            println!("✅ 已清空当前会话历史\n");
            false
        }
        "/resume" => {
            if let Some(id) = parts.get(1) {
                if session_manager.switch(id) {
                    println!("✅ 已切换到会话：{}\n", id);
                } else {
                    println!("❌ 会话不存在：{}\n", id);
                }
            } else {
                print_session_list(session_manager);
            }
            false
        }
        "/help" | "/h" => {
            print_interactive_help();
            false
        }
        _ => {
            println!("❌ 未知命令：{}", cmd);
            println!("输入 /help 查看帮助\n");
            false
        }
    }
}

/// 打印会话列表
fn print_session_list(session_manager: &SessionManager) {
    let sessions = session_manager.list();

    if sessions.is_empty() {
        println!("📭 暂无会话\n");
        return;
    }

    println!("📋 会话列表:");
    println!();

    let current_id = session_manager.current_session_id();

    for (id, metadata) in sessions {
        let name_str = metadata.name.as_deref().unwrap_or("(未命名)");
        let msgs = metadata.message_count;
        let short_id = if id.len() > 8 { &id[..8] } else { id };
        let marker = if current_id == Some(id) { "👉" } else { "  " };
        println!("{} {} - {} ({}条消息)", marker, short_id, name_str, msgs);
    }
    println!();
}

/// 设置 Agent 和 SessionManager
fn setup_agent(config: &Config, verbose: bool) -> Result<(Agent, SessionManager)> {
    let mut session_manager = SessionManager::new(config.session.storage_path.clone());
    session_manager.load_all()?;
    if session_manager.current().is_none() {
        session_manager.create(None, config.agent.clone());
    }

    let llm_client = LlmClient::new(config.agent.clone());
    let tool_executor = ToolExecutor::new(config.workspace.root.clone());

    let agent = Agent::new(llm_client, tool_executor, config.agent.clone(), verbose);

    Ok((agent, session_manager))
}

/// 显示会话状态
fn display_session_status(session_manager: &SessionManager) {
    if let Some(session_id) = session_manager.current_session_id() {
        let short_id = if session_id.len() > 8 { &session_id[..8] } else { session_id };
        if let Some(session) = session_manager.current() {
            let msg_count = session.context().len();
            println!("📝 会话：{} ({}条消息)", short_id, msg_count);
        } else {
            println!("📝 会话：{}", short_id);
        }
        println!();
    }
}

/// Agent 命令 - 交互式对话
async fn run_agent(verbose: bool) -> Result<()> {
    let config = Config::load_default()?;
    config.ensure_workspace()?;
    config.ensure_sessions()?;

    // 打印欢迎信息
    println!("╔════════════════════════════════════════╗");
    println!("║   🤖 rox - 本地 AI 助手                ║");
    println!("║   模型：{:<24} ║", truncate_str(&config.agent.model, 24));
    if verbose {
        println!("║   模式：详细日志                      ║");
    }
    println!("╚════════════════════════════════════════╝");
    println!();
    println!("💡 输入 /help 查看命令，/quit 退出");
    println!();

    // 设置 Agent 和 SessionManager
    let (mut agent, mut session_manager) = setup_agent(&config, verbose)?;

    // 显示当前会话信息
    display_session_status(&session_manager);

    // 显示历史消息（最近 5 条）
    print_session_history(&session_manager, 5);

    // 设置 readline
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
                    if handle_command(&mut session_manager, input) {
                        break;
                    }
                    continue;
                }

                // 普通对话
                match agent.chat(&mut session_manager, input).await {
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
                println!("\n输入 /quit 退出");
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
        "agent" | "a" => {
            let verbose = args.iter().any(|arg| arg == "--log" || arg == "-v" || arg == "--verbose");
            run_agent(verbose).await
        }
        "onboard" => run_onboard(),
        "help" | "-h" | "--help" | "h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("❌ 未知命令：{}", command);
            eprintln!("运行 'rox help' 查看帮助信息");
            std::process::exit(1);
        }
    }
}
