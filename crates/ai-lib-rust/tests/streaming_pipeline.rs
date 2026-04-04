use ai_lib_rust::types::events::StreamingEvent;
use ai_lib_rust::AiClient;
use futures::StreamExt;

#[tokio::test]
async fn test_openai_streaming_event_mapping() {
    let protocol_dir = "D:\\ai-protocol";
    std::env::set_var("AI_PROTOCOL_DIR", protocol_dir);

    let client = AiClient::new("openai/gpt-4o").await.unwrap();
    let pipeline = client.pipeline.clone();

    // Mock raw SSE data for OpenAI
    let chunks = vec![
        "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\" World\"},\"index\":0}]}\n\n",
        "data: [DONE]\n\n",
    ];

    let bytes_stream = futures::stream::iter(chunks)
        .map(|s| Ok::<bytes::Bytes, ai_lib_rust::Error>(bytes::Bytes::from(s)));

    let mut event_stream = pipeline
        .process_stream(Box::pin(bytes_stream))
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = event_stream.next().await {
        events.push(event.unwrap());
    }

    // Verify events
    // 1st chunk often ignored or metadata if it has no content/tools
    // 2nd chunk: PartialContentDelta "Hello"
    // 3rd chunk: PartialContentDelta " World"
    // 4th chunk: StreamEnd

    let content_deltas: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let StreamingEvent::PartialContentDelta { content, .. } = e {
                Some(content.clone())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(content_deltas, vec!["Hello", " World"]);
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamingEvent::StreamEnd { .. })));
}

#[tokio::test]
async fn test_deepseek_streaming_event_mapping() {
    let protocol_dir = "D:\\ai-protocol";
    std::env::set_var("AI_PROTOCOL_DIR", protocol_dir);

    // DeepSeek now has a complete event_map from my previous edit!
    let client = AiClient::new("deepseek/deepseek-chat").await.unwrap();
    let pipeline = client.pipeline.clone();

    let chunks = vec![
        "data: {\"choices\":[{\"delta\":{\"content\":\"Deep\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Seek\"},\"index\":0}]}\n\n",
        "data: [DONE]\n\n",
    ];

    let bytes_stream = futures::stream::iter(chunks)
        .map(|s| Ok::<bytes::Bytes, ai_lib_rust::Error>(bytes::Bytes::from(s)));

    let mut event_stream = pipeline
        .process_stream(Box::pin(bytes_stream))
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = event_stream.next().await {
        events.push(event.unwrap());
    }

    let content_deltas: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let StreamingEvent::PartialContentDelta { content, .. } = e {
                Some(content.clone())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(content_deltas, vec!["Deep", "Seek"]);
}
