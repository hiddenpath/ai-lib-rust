//! Plugin registry.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::base::{Plugin, PluginContext};
use crate::Result;

pub struct PluginRegistry { plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>, enabled: RwLock<bool> }

impl PluginRegistry {
    pub fn new() -> Self { Self { plugins: RwLock::new(HashMap::new()), enabled: RwLock::new(true) } }

    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        plugin.on_register().await?;
        self.plugins.write().unwrap().insert(name, plugin);
        Ok(())
    }

    pub async fn unregister(&self, name: &str) -> Result<Option<Arc<dyn Plugin>>> {
        let plugin = self.plugins.write().unwrap().remove(name);
        if let Some(ref p) = plugin { p.on_unregister().await?; }
        Ok(plugin)
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> { self.plugins.read().unwrap().get(name).cloned() }
    pub fn has(&self, name: &str) -> bool { self.plugins.read().unwrap().contains_key(name) }
    pub fn list(&self) -> Vec<Arc<dyn Plugin>> { self.plugins.read().unwrap().values().cloned().collect() }
    pub fn list_by_priority(&self) -> Vec<Arc<dyn Plugin>> { let mut p = self.list(); p.sort_by_key(|x| x.priority()); p }
    pub fn count(&self) -> usize { self.plugins.read().unwrap().len() }
    pub fn set_enabled(&self, e: bool) { *self.enabled.write().unwrap() = e; }
    pub fn is_enabled(&self) -> bool { *self.enabled.read().unwrap() }

    pub async fn trigger_before_request(&self, ctx: &mut PluginContext) -> Result<()> {
        if !self.is_enabled() { return Ok(()); }
        for p in self.list_by_priority() { if ctx.should_skip() { break; } p.on_before_request(ctx).await?; }
        Ok(())
    }

    pub async fn trigger_after_response(&self, ctx: &mut PluginContext) -> Result<()> {
        if !self.is_enabled() { return Ok(()); }
        for p in self.list_by_priority() { if ctx.should_skip() { break; } p.on_after_response(ctx).await?; }
        Ok(())
    }

    pub async fn trigger_on_error(&self, ctx: &mut PluginContext) -> Result<()> {
        if !self.is_enabled() { return Ok(()); }
        for p in self.list_by_priority() { p.on_error(ctx).await?; }
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let plugins: HashMap<_, _> = std::mem::take(&mut *self.plugins.write().unwrap());
        for (_, p) in plugins { let _ = p.on_unregister().await; }
        Ok(())
    }
}
impl Default for PluginRegistry { fn default() -> Self { Self::new() } }

static GLOBAL_REGISTRY: once_cell::sync::Lazy<PluginRegistry> = once_cell::sync::Lazy::new(PluginRegistry::new);
pub fn get_plugin_registry() -> &'static PluginRegistry { &GLOBAL_REGISTRY }
