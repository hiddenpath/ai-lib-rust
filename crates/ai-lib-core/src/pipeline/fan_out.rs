use crate::pipeline::{PipelineError, Transform};
use crate::protocol::CandidateConfig;
use crate::{BoxStream, PipeResult};
use futures::StreamExt;
use serde_json::Value;

/// FanOut replicates the stream or splits array elements into separate events
pub struct FanOut {
    config: CandidateConfig,
}

impl FanOut {
    pub fn new(config: CandidateConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Transform for FanOut {
    async fn transform(
        &self,
        input: BoxStream<'static, Value>,
    ) -> PipeResult<BoxStream<'static, Value>> {
        let fan_out_enabled = self.config.fan_out.unwrap_or(false);

        if !fan_out_enabled {
            return Ok(input);
        }

        // If FanOut is enabled, we assume the input might be an Array of candidates
        // and we want to emit each candidate as a separate item in the stream.
        // Or if it's an object, we pass it through.

        // Note: Real fan-out in async streams often implies parallel request processing,
        // but in the pipeline context, it usually means "One Event Frame -> Many Event Frames"

        let stream = input.flat_map(|result| {
            let res_vec: Vec<Result<Value, crate::Error>> = match result {
                Ok(Value::Array(arr)) => arr.into_iter().map(Ok).collect(),
                Ok(val) => vec![Ok(val)],
                Err(e) => vec![Err(e)],
            };
            futures::stream::iter(res_vec)
        });

        Ok(Box::pin(stream))
    }
}

pub fn create_fan_out(config: &CandidateConfig) -> Result<Box<dyn Transform>, PipelineError> {
    Ok(Box::new(FanOut::new(config.clone())))
}
