# 三项任务实施计划

## 任务 1: 在 ai-protocol 库中添加 Mistral Provider 和 Models

### 1.1 研究 Mistral API 文档
**参考:** https://docs.mistral.ai/api

**关键信息:**
- Base URL: `https://api.mistral.ai/v1`
- 认证: Bearer token via `MISTRAL_API_KEY` 环境变量
- 主要端点: `POST /v1/chat/completions`
- API 格式: OpenAI 兼容格式
- 流式响应: SSE 格式，与 OpenAI 相同
- 支持的参数: model, messages, temperature, max_tokens, stream, tools, tool_choice, top_p, frequency_penalty, presence_penalty, random_seed, stop, n, parallel_tool_calls, safe_prompt, response_format, prompt_mode, prediction, metadata
- 响应格式: OpenAI 兼容，包含 `choices`, `usage`, `id`, `model`, `created`, `object`
- 流式事件: SSE 格式，`data: [DONE]` 结束

### 1.2 创建 Mistral Provider 配置
**文件:** `D:\ai-protocol\v1\providers\mistral.yaml`

**需要包含:**
- Provider ID: `mistral`
- Base URL: `https://api.mistral.ai/v1`
- 认证配置: Bearer token
- 端点配置: `/chat/completions`
- 参数映射: OpenAI 兼容
- 流式配置: SSE 格式
- 终止原因映射
- 工具调用映射
- 重试策略
- 速率限制头

### 1.3 创建 Mistral Models 配置
**文件:** `D:\ai-protocol\v1\models\mistral.yaml`

**需要包含的模型:**
- `mistral-small-latest`
- `mistral-medium-latest`
- `mistral-large-latest`
- `mistral-tiny`
- `pixtral-small-latest`
- `pixtral-large-latest`
- `open-mistral-7b`
- `open-mixtral-8x7b`
- `mistral-7b-instruct`

### 1.4 验证和提交
- 验证 YAML 格式符合 schema
- 测试配置加载
- 提交 PR 到 ai-protocol 仓库

---

## 任务 2: 在 ai-lib-rust 中实现统一的结构化 Metrics 功能

### 2.1 设计 Usage 结构体
**位置:** `src/types/usage.rs` (新建)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    // 扩展字段（可选）
    pub cached_tokens: Option<u32>,
}
```

### 2.2 实现 Usage 解析逻辑
- 从 `serde_json::Value` 解析到结构化 `Usage`
- 处理不同 provider 的字段名差异（如 `totalTokens` vs `total_tokens`）
- 提供默认值和错误处理

### 2.3 更新 StreamingEvent
**文件:** `src/types/events.rs`
- 将 `StreamingEvent::Metadata` 中的 `usage: Option<serde_json::Value>` 改为 `usage: Option<Usage>`

### 2.4 更新 UnifiedResponse
**文件:** `src/client/core.rs`
- 将 `usage: Option<serde_json::Value>` 改为 `usage: Option<Usage>`

### 2.5 更新 Pipeline 和 Transport
- 在事件映射时解析 usage JSON 到结构化类型
- 更新所有使用 usage 的地方

### 2.6 检查 ai-protocol Schema
- 检查是否需要更新 schema 以支持结构化 usage
- 如果需要，提交 PR 到 ai-protocol

---

## 任务 3: 移除 Facade 层次

### 3.1 分析 Facade 层功能
**当前 Facade 模块:**
- `src/facade/provider.rs`: Provider 枚举, ModelRef, client_from_provider
- `src/facade/chat.rs`: ChatCompletionRequest, ChatFacade trait
- `src/facade/prelude.rs`: 重新导出

**功能分析:**
1. **Provider 枚举**: 提供类型安全的 provider 标识
2. **ModelRef**: 提供 provider/model 字符串构建
3. **ChatCompletionRequest**: 提供开发者友好的请求构建器
4. **ChatFacade trait**: 在 AiClient 上提供便捷方法

### 3.2 迁移策略

#### 3.2.1 Provider 和 ModelRef
**迁移到:** `src/client/provider.rs` (新建)
- 将 Provider 枚举移到 client 模块
- 将 ModelRef 移到 client 模块
- 更新所有导入

#### 3.2.2 ChatCompletionRequest
**选项 A:** 直接使用 `ChatRequestBuilder` (推荐)
- 移除 `ChatCompletionRequest`
- 直接使用 `client.chat().messages(...).temperature(...).execute()`

**选项 B:** 保留但移到 client 模块
- 将 `ChatCompletionRequest` 移到 `src/client/chat.rs`
- 作为 `ChatRequestBuilder` 的便捷包装

#### 3.2.3 ChatFacade 方法
**迁移到:** `AiClient` impl 块
- 将 `chat_completion`, `chat_completion_stream` 等方法直接添加到 `AiClient`
- 移除 trait，改为直接方法

### 3.3 更新代码
1. 更新 `src/lib.rs` 导出
2. 更新所有 examples
3. 更新所有 tests
4. 更新 aitest 项目

### 3.4 删除 Facade 模块
- 删除 `src/facade/` 目录
- 更新文档

---

## 执行顺序

1. **任务 1** (ai-protocol): 先完成，因为其他任务可能依赖
2. **任务 2** (结构化 Usage): 可以并行或稍后进行
3. **任务 3** (移除 Facade): 最后进行，因为会影响所有使用代码

---

## 预期影响

### 任务 1 影响
- ✅ 新增 Mistral provider 支持
- ✅ 新增多个 Mistral 模型配置
- ⚠️ 需要更新 ai-lib-rust 的 Provider 枚举（添加 Mistral 变体）

### 任务 2 影响
- ✅ 更好的类型安全
- ✅ 更易用的 API
- ⚠️ 可能需要更新所有使用 usage 的代码

### 任务 3 影响
- ✅ 简化代码结构
- ✅ 减少抽象层次
- ⚠️ 需要更新所有使用 facade 的代码（包括 examples, tests, aitest）
