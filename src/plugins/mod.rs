//! Plugin and middleware system.

mod base;
mod hooks;
mod middleware;
mod registry;

pub use base::{Plugin, PluginContext, PluginPriority, CompositePlugin};
pub use hooks::{Hook, HookType, HookManager, AsyncHook, FnHook};
pub use middleware::{Middleware, MiddlewareChain, MiddlewareContext};
pub use registry::{PluginRegistry, get_plugin_registry};
