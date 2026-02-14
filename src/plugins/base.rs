//! Base plugin types.

use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginPriority {
    Highest = 0,
    High = 25,
    Normal = 50,
    Low = 75,
    Lowest = 100,
}
impl Default for PluginPriority {
    fn default() -> Self {
        PluginPriority::Normal
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginContext {
    pub request: Option<serde_json::Value>,
    pub response: Option<serde_json::Value>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub skip: bool,
}

impl PluginContext {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_request(mut self, r: serde_json::Value) -> Self {
        self.request = Some(r);
        self
    }
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
    pub fn skip(&mut self) {
        self.skip = true;
    }
    pub fn should_skip(&self) -> bool {
        self.skip
    }
    pub fn set_error(&mut self, e: impl Into<String>) {
        self.error = Some(e.into());
    }
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> PluginPriority {
        PluginPriority::Normal
    }
    async fn on_register(&self) -> Result<()> {
        Ok(())
    }
    async fn on_unregister(&self) -> Result<()> {
        Ok(())
    }
    async fn on_before_request(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
    async fn on_after_response(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
    async fn on_error(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
    async fn on_stream_event(
        &self,
        _ctx: &mut PluginContext,
        _event: &serde_json::Value,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct CompositePlugin {
    name: String,
    plugins: Vec<Arc<dyn Plugin>>,
}
impl CompositePlugin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            plugins: Vec::new(),
        }
    }
    pub fn add(mut self, p: Arc<dyn Plugin>) -> Self {
        self.plugins.push(p);
        self
    }
    pub fn len(&self) -> usize {
        self.plugins.len()
    }
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[async_trait]
impl Plugin for CompositePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    async fn on_register(&self) -> Result<()> {
        for p in &self.plugins {
            p.on_register().await?;
        }
        Ok(())
    }
    async fn on_unregister(&self) -> Result<()> {
        for p in &self.plugins {
            p.on_unregister().await?;
        }
        Ok(())
    }
    async fn on_before_request(&self, ctx: &mut PluginContext) -> Result<()> {
        for p in &self.plugins {
            if ctx.should_skip() {
                break;
            }
            p.on_before_request(ctx).await?;
        }
        Ok(())
    }
    async fn on_after_response(&self, ctx: &mut PluginContext) -> Result<()> {
        for p in &self.plugins {
            if ctx.should_skip() {
                break;
            }
            p.on_after_response(ctx).await?;
        }
        Ok(())
    }
    async fn on_error(&self, ctx: &mut PluginContext) -> Result<()> {
        for p in &self.plugins {
            p.on_error(ctx).await?;
        }
        Ok(())
    }
}
