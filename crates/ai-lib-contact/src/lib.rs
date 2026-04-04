//! # ai-lib-contact
//!
//! 策略与横切能力层：缓存、批处理、路由、插件、拦截器、令牌、遥测、护栏、弹性（熔断/限流）。
//! 依赖 `ai-lib-core` 执行层类型与错误。
//!
//! Policy and cross-cutting modules for AI-Protocol. Depends on `ai-lib-core`.

pub mod cache;
pub mod plugins;
pub mod resilience;

#[cfg(feature = "batch")]
pub mod batch;
#[cfg(feature = "guardrails")]
pub mod guardrails;
#[cfg(feature = "tokens")]
pub mod tokens;
#[cfg(feature = "telemetry")]
pub mod telemetry;
#[cfg(feature = "routing_mvp")]
pub mod routing;
#[cfg(feature = "interceptors")]
pub mod interceptors;
