# brk

一个用 Rust 编写的 CLI Agent，通过 Ollama API 与本地 LLM 交互，支持内置工具调用和会话管理。

## 功能特性

- 🤖 与 Ollama 本地模型对话
- 🔧 内置工具支持：
  - `fs_read` / `fs_write` / `fs_patch` / `fs_list` - 文件系统操作
  - `web_search` / `web_fetch` - 网络搜索和网页抓取
  - `get_time` - 获取当前时间
- 🔄 自动工具调用循环
- 🛡️ LLM 调用重试机制
- 💾 会话管理 - 支持多会话和持久化
- 📝 可配置的系统提示 - 支持 AGENT.md、SOUL.md、USER.md

## 快速开始

### 构建

```bash
cargo build
cargo build --release    # 发布构建
```

### 运行

```bash
cargo run
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OLLAMA_MODEL` | `qwen3:4b-instruct-2507-q4_K_M` | Ollama 模型名称 |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API 地址 |
| `TAVILY_API_KEY` | - | web_search 所需 API 密钥 |

```bash
OLLAMA_MODEL=llama2 OLLAMA_URL=http://192.168.1.100:11434 cargo run
```

## 项目结构

```
src/
├── main.rs              # 程序入口
├── lib.rs               # 库导出
├── cli.rs               # CLI 交互
├── types/               # 类型定义
│   ├── function.rs      # 函数相关类型
│   └── ollama.rs        # Ollama API 类型
├── agent/               # Agent 核心
│   ├── config.rs        # 配置参数
│   ├── context.rs       # 上下文管理（系统提示 + 消息历史）
│   ├── session.rs       # 会话管理（CRUD + 持久化）
│   ├── llm.rs           # LLM 通信客户端
│   └── core.rs          # Agent 状态与流程
└── tools/               # 工具系统
    ├── registry.rs      # 工具注册
    ├── executor.rs      # 工具执行器
    ├── builtins/        # 内置工具实现
    │   ├── fs.rs        # 文件系统工具
    │   ├── web.rs       # 网络工具
    │   └── get_time.rs  # 时间工具
    └── impls/           # 工具具体实现
```

## 配置文件

在 `~/.brk/` 目录下创建配置文件来自定义系统提示：

```bash
~/.brk/
├── AGENT.md    # 角色定义
├── SOUL.md     # 对话风格
└── USER.md     # 用户偏好
```

## 许可证

MIT
