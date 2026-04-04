//! Plugin and middleware system.

mod base;
mod hooks;
mod middleware;
mod registry;

pub use base::{CompositePlugin, Plugin, PluginContext, PluginPriority};
pub use hooks::{AsyncHook, FnHook, Hook, HookManager, HookType};
pub use middleware::{Middleware, MiddlewareChain, MiddlewareContext};
pub use registry::{get_plugin_registry, PluginRegistry};
