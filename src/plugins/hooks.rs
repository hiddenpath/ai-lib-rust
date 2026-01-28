//! Hook system.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::base::PluginContext;
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookType { BeforeRequest, AfterResponse, OnError, OnStreamEvent, OnRetry, OnFallback, OnCacheHit, OnCacheMiss }

#[async_trait]
pub trait AsyncHook: Send + Sync { async fn call(&self, ctx: &mut PluginContext) -> Result<()>; }

pub struct Hook { pub name: String, pub priority: i32, callback: Arc<dyn AsyncHook> }
impl Hook {
    pub fn new<H: AsyncHook + 'static>(name: impl Into<String>, priority: i32, callback: H) -> Self { Self { name: name.into(), priority, callback: Arc::new(callback) } }
    pub async fn call(&self, ctx: &mut PluginContext) -> Result<()> { self.callback.call(ctx).await }
}

pub struct FnHook<F> { func: F }
impl<F> FnHook<F> where F: Fn(&mut PluginContext) -> Result<()> + Send + Sync { pub fn new(func: F) -> Self { Self { func } } }
#[async_trait]
impl<F> AsyncHook for FnHook<F> where F: Fn(&mut PluginContext) -> Result<()> + Send + Sync { async fn call(&self, ctx: &mut PluginContext) -> Result<()> { (self.func)(ctx) } }

pub struct HookManager { hooks: RwLock<HashMap<HookType, Vec<Hook>>> }
impl HookManager {
    pub fn new() -> Self { Self { hooks: RwLock::new(HashMap::new()) } }

    pub fn register(&self, hook_type: HookType, hook: Hook) {
        let mut hooks = self.hooks.write().unwrap();
        let entry = hooks.entry(hook_type).or_insert_with(Vec::new);
        entry.push(hook);
        entry.sort_by_key(|h| h.priority);
    }

    pub fn register_fn<F>(&self, hook_type: HookType, name: impl Into<String>, priority: i32, func: F)
    where F: Fn(&mut PluginContext) -> Result<()> + Send + Sync + 'static {
        self.register(hook_type, Hook::new(name, priority, FnHook::new(func)));
    }

    pub fn unregister(&self, hook_type: HookType, name: &str) -> bool {
        let mut hooks = self.hooks.write().unwrap();
        if let Some(entry) = hooks.get_mut(&hook_type) { let len = entry.len(); entry.retain(|h| h.name != name); return entry.len() < len; }
        false
    }

    pub async fn trigger(&self, hook_type: HookType, ctx: &mut PluginContext) -> Result<()> {
        let callbacks: Vec<Arc<dyn AsyncHook>> = { let hooks = self.hooks.read().unwrap(); hooks.get(&hook_type).map(|v| v.iter().map(|h| h.callback.clone()).collect()).unwrap_or_default() };
        for cb in callbacks { if ctx.should_skip() { break; } cb.call(ctx).await?; }
        Ok(())
    }

    pub fn count(&self, hook_type: HookType) -> usize { self.hooks.read().unwrap().get(&hook_type).map(|v| v.len()).unwrap_or(0) }
    pub fn clear(&self) { self.hooks.write().unwrap().clear(); }
}
impl Default for HookManager { fn default() -> Self { Self::new() } }
