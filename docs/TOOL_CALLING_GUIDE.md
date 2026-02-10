# Tool Calling with Tavily Search - 使用指南

完整的Tool Calling示例，展示如何与支持函数调用的大模型集成Tavily搜索工具。

## 快速开始 (5分钟)

### 1. 设置API密钥
```powershell
# 推荐使用DeepSeek
$env:DEEPSEEK_API_KEY="your_api_key"

# 或使用其他提供商
$env:OPENAI_API_KEY="your_openai_key"
$env:ANTHROPIC_API_KEY="your_anthropic_key"
$env:GROQ_API_KEY="your_groq_key"
```

### 2. 运行示例
```powershell
cd d:\rustapp\ai-lib-rust
cargo run --example tavily_tool_calling

# 指定提供商
cargo run --example tavily_tool_calling -- --provider openai

# 启用调试日志
$env:RUST_LOG="ai_lib_rust::pipeline=debug"
cargo run --example tavily_tool_calling
```

### 3. 使用启动脚本
```powershell
d:\rustapp\ai-lib-rust\examples\run_tavily_example.ps1
```

---

## 完整工作流程

### Phase 1: 工具定义
定义Tavily Search工具的结构和参数：
```rust
fn tavily_search_tool() -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "tavily_search".to_string(),
            description: Some("Search the web using Tavily API".to_string()),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "search_depth": { "enum": ["basic", "advanced"], "default": "basic" },
                    "max_results": { "type": "integer", "minimum": 1, "maximum": 10, "default": 5 }
                },
                "required": ["query"]
            })),
        },
    }
}
```

### Phase 2: 初始请求
发送包含工具定义的请求，强制模型使用工具：
```rust
let client = AiClient::new("deepseek/auto").await?;

let messages = vec![
    Message::system("You are a helpful research assistant."),
    Message::user("What are the latest developments in Rust?"),
];

let tool_choice = json!({
    "type": "function",
    "function": { "name": "tavily_search" }
});

let resp = client
    .chat()
    .messages(messages)
    .tools(vec![tavily_search_tool()])
    .tool_choice(tool_choice)
    .execute()
    .await?;

// resp.tool_calls 包含模型请求的工具调用
```

### Phase 3: 处理工具调用
接收并执行模型请求的工具：
```rust
pub async fn process_tool_calls(tool_calls: &[ToolCall]) 
    -> Result<Vec<ToolResult>, Box<dyn std::error::Error>> {
    
    let mut results = Vec::new();
    for tool_call in tool_calls {
        if tool_call.name == "tavily_search" {
            let query = tool_call.arguments.get("query")?.as_str()?;
            let depth = tool_call.arguments.get("search_depth")?.as_str().unwrap_or("basic");
            let max_results = tool_call.arguments.get("max_results")?.as_i64().unwrap_or(5) as i32;
            
            let search_results = mock_tavily_search(query, depth, max_results).await?;
            
            results.push(ToolResult {
                tool_use_id: tool_call.id.clone(),
                content: search_results,
                is_error: false,
            });
        }
    }
    Ok(results)
}
```

### Phase 4: 回传结果并获得最终响应
构建包含工具结果的消息，获取模型的最终响应：
```rust
let mut follow_up_messages = vec![
    Message::system("You are a helpful research assistant."),
    Message::user("What are the latest developments in Rust?"),
];

// 添加助手的工具调用请求
let mut assistant_blocks = vec![ContentBlock::text(&resp.content)];
for tool_call in &resp.tool_calls {
    assistant_blocks.push(ContentBlock::ToolUse {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        input: tool_call.arguments.clone(),
    });
}
follow_up_messages.push(Message::with_content(
    MessageRole::Assistant,
    MessageContent::blocks(assistant_blocks),
));

// 添加工具结果
for result in tool_results {
    follow_up_messages.push(Message::with_content(
        MessageRole::User,
        MessageContent::blocks(vec![
            ContentBlock::ToolResult {
                tool_use_id: result.tool_use_id,
                content: result.content,
            },
        ]),
    ));
}

// 获得最终响应
let final_resp = client
    .chat()
    .messages(follow_up_messages)
    .execute()
    .await?;

println!("Assistant: {}", final_resp.content);
```

---

## 关键概念

### ContentBlock 类型
用于在messages中表现不同类型的内容：

| 类型 | 说明 | 用途 |
|------|------|------|
| `ContentBlock::Text` | 纯文本内容 | 消息主体 |
| `ContentBlock::ToolUse` | 工具调用请求 | 助手请求执行工具 |
| `ContentBlock::ToolResult` | 工具执行结果 | 回传工具结果给模型 |
| `ContentBlock::Image` | 图片内容 | 多模态支持 |
| `ContentBlock::Audio` | 音频内容 | 音频处理 |

### ToolCall 结构
```rust
pub struct ToolCall {
    pub id: String,                    // 唯一调用ID
    pub name: String,                  // 工具名称  
    pub arguments: serde_json::Value,  // JSON格式参数
}
```

### ToolResult 结构
```rust
pub struct ToolResult {
    pub tool_use_id: String,           // 对应的工具调用ID
    pub content: serde_json::Value,    // 执行结果
    pub is_error: bool,                // 是否出错
}
```

---

## 多模型支持

支持的提供商（自动检测优先级）：

```
1. DEEPSEEK_API_KEY       ✅ 推荐，最佳支持
2. OPENAI_API_KEY         ✅ 稳定，广泛使用
3. ANTHROPIC_API_KEY      ✅ 支持良好
4. GROQ_API_KEY          ✅ 免费选项
```

自动检测逻辑：
```rust
fn get_provider() -> String {
    // 检查命令行参数
    if let Some(provider) = args_provider {
        return format!("{}/auto", provider);
    }
    
    // 检查环境变量
    if env::var("DEEPSEEK_API_KEY").is_ok() {
        "deepseek/auto".to_string()
    } else if env::var("OPENAI_API_KEY").is_ok() {
        "openai/auto".to_string()
    } // ...
}
```

---

## 常见问题

### Q: 模型没有调用工具？
A: 检查以下几点：
- 确保使用支持tool calling的模型（DeepSeek、OpenAI推荐）
- 确认tool_choice正确设置为强制使用
- 系统提示中清楚描述工具的用途
- 某些模型需要特定的温度设置（试试设置为0.0）

### Q: 工具调用参数格式错误？
A: 
- 检查JSON Schema定义是否正确
- 相应参数类型是否与schema匹配
- 必填字段是否都提供了

### Q: 如何添加新工具？
A: 
1. 定义新的ToolDefinition（参考tavily_search_tool）
2. 在process_tool_calls中添加匹配分支
3. 向tools vector中添加新工具

### Q: 如何调试？
A:
```powershell
$env:RUST_LOG="ai_lib_rust::pipeline=debug"
cargo run --example tavily_tool_calling
```

---

## 架构设计

```
┌─────────────────────────────────────────────┐
│          Tool Calling Flow                  │
├─────────────────────────────────────────────┤
│                                             │
│  User Request                               │
│  (+ Tool Definitions)                       │
│         ↓                                   │
│  ┌─────────────────────────────────────┐   │
│  │ AiClient.chat()                     │   │
│  │ - messages()                        │   │
│  │ - tools()                           │   │
│  │ - tool_choice()                     │   │
│  │ - execute()                         │   │
│  └─────────────────────────────────────┘   │
│         ↓                                   │
│  Model Response with Tool Calls             │
│  (resp.tool_calls: Vec<ToolCall>)           │
│         ↓                                   │
│  Process Tool Calls                         │
│  for each call:                             │
│    - Extract parameters                    │
│    - Execute tool logic                    │
│    - Return ToolResult                     │
│         ↓                                   │
│  Follow-up Request                          │
│  (Original messages + Assistant's           │
│   tool calls + Tool results)                │
│         ↓                                   │
│  Final Response from Model                  │
│  (Processed based on tool results)          │
│         ↓                                   │
│  User gets AI-generated summary             │
│                                             │
└─────────────────────────────────────────────┘
```

---

## 实现示例

### Mock搜索实现
```rust
async fn mock_tavily_search(
    query: &str,
    _depth: &str,
    _max_results: i32,
) -> Result<Value, Box<dyn std::error::Error>> {
    println!("🔍 Searching for: {}", query);
    
    Ok(json!({
        "results": [
            {
                "title": "Result 1",
                "url": "https://example.com",
                "content": "Description..."
            }
        ],
        "query": query
    }))
}
```

### 实际集成
如需集成真实Tavily API：
```rust
async fn real_tavily_search(query: &str, ...) -> Result<Value, ...> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.tavily.com/search")
        .json(&json!({
            "api_key": env::var("TAVILY_API_KEY")?,
            "query": query,
            ...
        }))
        .send()
        .await?;
    Ok(response.json().await?)
}
```

---

## 预期输出示例

```
🚀 Tavily Search Tool Calling Example

📦 Using provider: deepseek/auto

📤 Sending initial request with tool definition...

✅ Initial response received
   Content: I'll search for the latest Rust developments...

🔄 Processing 1 tool call(s)...

📌 Tool Call: tavily_search
   ID: call_xyz789
   Arguments: {"query": "latest Rust 2025", ...}

🔍 Executing Tavily search for: latest Rust 2025

📤 Sending tool results back to model...

✅ Final response received

📝 Assistant Response:
Based on the latest search results, here are the key developments...

📊 Token Usage:
   Prompt tokens: 1234
   Completion tokens: 567

✨ Example completed successfully!
```

---

## 扩展方向

### 短期扩展
- 集成真实Tavily API调用
- 添加更多工具（计算器、天气、代码搜索）
- 参数验证增强

### 中期扩展
- 多轮对话支持
- 并行工具执行
- 工具超时控制
- 执行记录持久化

### 长期扩展
- 工具编排和链式调用
- 动态工具发现
- 使用统计和优化
- 工具版本管理

---

## 技术细节

### 支持的参数类型
工具定义支持JSON Schema常见类型：
- `string` - 文本参数
- `integer` - 整数参数
- `number` - 浮点数参数
- `boolean` - 布尔参数
- `array` - 数组参数
- `object` - 对象参数
- `enum` - 枚举选项

### Message 构建最佳实践
1. 系统消息：定义助手角色和行为
2. 用户消息：提供初始请求
3. 助手消息：包含ContentBlock::ToolUse
4. 用户消息：包含ContentBlock::ToolResult

### 错误处理
- 缺失环境变量检查
- 网络错误捕获
- 工具执行异常处理
- 参数验证错误

---

## 参考资源

- [AI-Protocol 标准](https://github.com/hiddenpath/ai-protocol)
- [Tavily API 文档](https://tavily.com/docs)
- [OpenAI Function Calling](https://platform.openai.com/docs/guides/function-calling)
- [DeepSeek API](https://platform.deepseek.com)

---

## 许可证

MIT OR Apache-2.0

**最后更新**: 2025-02-09  
**版本**: 1.0
