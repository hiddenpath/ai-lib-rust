#[cfg(test)]
mod tests {
    use crate::pipeline::{fan_out, select, Transform};
    use crate::protocol::CandidateConfig;
    use futures::StreamExt;
    use serde_json::json;

    #[tokio::test]
    async fn test_selector_filtering() {
        let selector = select::Selector::new("choices.0.delta.content".to_string());

        // Input: Stream of simple frames simulating a chunked response
        let input_data = vec![
            json!({
                "choices": [{"delta": {"content": "Hello"}}]
            }),
            json!({
                "choices": [{"delta": {"content": " World"}}]
            }),
            json!({
                "choices": [] // Should be filtered out
            }),
            json!({"other": "ignored"}), // Should be filtered out
        ];

        let input_stream = futures::stream::iter(input_data).map(Ok);
        let output_stream = selector.transform(Box::pin(input_stream)).await.unwrap();

        let results: Vec<_> = output_stream.map(|r| r.unwrap()).collect().await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], json!("Hello"));
        assert_eq!(results[1], json!(" World"));
    }

    #[tokio::test]
    async fn test_fan_out() {
        let config = CandidateConfig {
            fan_out: Some(true),
            candidate_id_path: None,
        };
        let fan_out = fan_out::FanOut::new(config);

        // Input: Stream with arrays
        let input_data = vec![
            json!(["Candidate A"]),
            json!(["Candidate B", "Candidate C"]),
        ];

        let input_stream = futures::stream::iter(input_data).map(Ok);
        let output_stream = fan_out.transform(Box::pin(input_stream)).await.unwrap();

        let results: Vec<_> = output_stream.map(|r| r.unwrap()).collect().await;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], json!("Candidate A"));
        assert_eq!(results[1], json!("Candidate B"));
        assert_eq!(results[2], json!("Candidate C"));
    }
}
