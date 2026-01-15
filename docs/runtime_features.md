# ai-lib-rust 运行时特性清单 (Feature List)

本文档从开发者用户角度总结了 `ai-lib-rust` 运行时的核心能力，展示了其作为 Protocol-Driven 型 AI 基础设施的优势。

## 1. 模型交互能力 (Model Interaction)
专注于底层协议到具体模型调用的标准化映射。

- **多供应商协议解耦**: 通过 Manifest 声明式定义 OpenAI, Anthropic, Gemini, DeepSeek 等主流供应商的 API 差异，实现统一调用。
- **流式映射引擎 (Stream Mapping)**: 支持跨供应商的 SSE/Data-lines 格式归一化，统一输出 `PartialContentDelta` 和 `ToolCallStarted` 事件。
- **多模态输入标准化**: 统一封装文本、图像（Base64/URL）、音频等复合消息结构（进行中）。
- **工具调用适配 (Function Calling)**: 标准化 OpenAI 风格与 Anthropic 风格的工具定义与响应组装。

## 2. 交互管理能力 (Interaction Management)
专注于请求生命周期的健壮性与可观测性。

- **自适应频率限制 (Adaptive Rate Limiting)**: 
  - 支持本地频率限制设置。
  - **自动感知**: 实时提取响应 Header（如 `x-ratelimit-remaining`）进行动态阻塞与等待时间预测。
- **并发控制 (In-flight Control)**: 通过信号量（Semaphore）限制全局或模型级别的最大瞬时请求数，防止单点过载。
- **超时保护**: 支持全局请求超时与单次重试尝试超时（Attempt Timeout）的双重保护，确保系统响应性。
- **会话上下文管理**: 提供便捷的 `ChatRequestBuilder` 流式接口，支持历史消息管理。

## 3. 交互策略规划能力 (Strategy & Decision)
专注于运行时的高级决策逻辑，提升业务连续性。

- **能力驱动路由 (Capability-aware Routing)**: 
  - 发起请求前自动根据 Manifest 校验模型能力（如是否支持 tools/vision）。
  - 不满足能力要求的候选模型将被自动过滤，确保请求 100% 合法。
- **分级回退策略 (Fallback Chain)**: 
  - 支持多级模型 Fallback 链路（如 GPT-4 故障时自动切换至 Claude-3.5）。
  - 智能合并 Fallback 逻辑与能力校验，实现静默降级。
- **熔断机制 (Circuit Breaker)**: 实现标准熔断器状态机（Closed, Open, Half-Open），基于连续失败数或超时率自动隔离不稳定的模型后端。
- **自适应重试 (Adaptive Retries)**: 基于 `retry_policy` 定义，自动处理 429 和 5xx 错误，支持指数回避（Exp Backoff）与全抖动（Full Jitter）。

## 4. 其他核心能力
- **本地化先行 (Local-First Validation)**: 在请求发出前通过本地 JSON Schema 进行合法性校验，拒绝无效请求。
- **热重载支持 (Hot-Reloading)**: 支持在不重启进程的情况下更新模型清单（Manifest），动态调整配置参数。
- **开发者体验**:
  - 极简的一行代码调用（`AiClient::new("provider/model").chat().messages(...)`）。
  - 完整的 `CallStats` 统计，包含延迟、令牌数、首包时间等性能元数据。
