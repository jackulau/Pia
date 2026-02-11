use serde::Deserialize;

/// Shared SSE (Server-Sent Events) stream parser for OpenAI-compatible APIs.
/// Used by OpenAI, OpenRouter, and any other provider using the `data: ` line protocol.

#[derive(Deserialize)]
pub struct SseStreamChunk {
    pub choices: Vec<SseStreamChoice>,
    #[serde(default)]
    pub usage: Option<SseUsageInfo>,
}

#[derive(Deserialize)]
pub struct SseStreamChoice {
    pub delta: Option<SseDeltaContent>,
}

#[derive(Deserialize)]
pub struct SseDeltaContent {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct SseUsageInfo {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

/// Result of processing buffered SSE data. Returned token counts are cumulative
/// (last usage event wins), so callers should overwrite rather than accumulate.
pub struct SseProcessResult {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Process all complete SSE lines in `buffer`, appending content to `full_response`
/// and invoking `on_chunk` for each content delta. Drains processed lines from the
/// buffer, leaving any trailing incomplete line for the next call.
///
/// Returns aggregated token usage from the last usage event seen (if any).
pub fn process_sse_buffer(
    buffer: &mut String,
    full_response: &mut String,
    on_chunk: &dyn Fn(&str),
) -> SseProcessResult {
    let mut result = SseProcessResult {
        input_tokens: None,
        output_tokens: None,
    };

    while let Some(pos) = buffer.find('\n') {
        {
            let line = &buffer[..pos];
            if let Some(data) = line.strip_prefix("data: ") {
                if data != "[DONE]" {
                    if let Ok(chunk) = serde_json::from_str::<SseStreamChunk>(data) {
                        for choice in &chunk.choices {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    full_response.push_str(content);
                                    on_chunk(content);
                                }
                            }
                        }

                        if let Some(usage) = &chunk.usage {
                            result.input_tokens = usage.prompt_tokens;
                            result.output_tokens = usage.completion_tokens;
                        }
                    }
                }
            }
        }
        buffer.drain(..pos + 1);
    }

    result
}

/// Append raw bytes to a string buffer, preferring zero-copy `from_utf8` and
/// falling back to `from_utf8_lossy` only when the chunk contains invalid UTF-8.
#[inline]
pub fn append_bytes_to_buffer(buffer: &mut String, bytes: &[u8]) {
    match std::str::from_utf8(bytes) {
        Ok(s) => buffer.push_str(s),
        Err(_) => buffer.push_str(&String::from_utf8_lossy(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_process_single_chunk() {
        let mut buffer = String::from(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n",
        );
        let mut response = String::new();
        let chunks = RefCell::new(Vec::new());

        let result = process_sse_buffer(&mut buffer, &mut response, &|c| {
            chunks.borrow_mut().push(c.to_string());
        });

        assert_eq!(response, "hello");
        assert_eq!(chunks.into_inner(), vec!["hello"]);
        assert!(result.input_tokens.is_none());
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_process_done_sentinel() {
        let mut buffer = String::from("data: [DONE]\n");
        let mut response = String::new();

        process_sse_buffer(&mut buffer, &mut response, &|_| {});

        assert_eq!(response, "");
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_process_usage_info() {
        let mut buffer = String::from(
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}\n",
        );
        let mut response = String::new();

        let result = process_sse_buffer(&mut buffer, &mut response, &|_| {});

        assert_eq!(result.input_tokens, Some(10));
        assert_eq!(result.output_tokens, Some(20));
    }

    #[test]
    fn test_partial_line_preserved() {
        let mut buffer = String::from(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\ndata: {\"choi",
        );
        let mut response = String::new();

        process_sse_buffer(&mut buffer, &mut response, &|_| {});

        assert_eq!(response, "hi");
        assert_eq!(buffer, "data: {\"choi");
    }

    #[test]
    fn test_multiple_lines() {
        let mut buffer = String::from(
            "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\ndata: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\n",
        );
        let mut response = String::new();
        let chunks = RefCell::new(Vec::new());

        process_sse_buffer(&mut buffer, &mut response, &|c| {
            chunks.borrow_mut().push(c.to_string());
        });

        assert_eq!(response, "ab");
        assert_eq!(chunks.into_inner(), vec!["a", "b"]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_empty_lines_skipped() {
        let mut buffer = String::from(
            "\ndata: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n",
        );
        let mut response = String::new();

        process_sse_buffer(&mut buffer, &mut response, &|_| {});

        assert_eq!(response, "x");
    }

    #[test]
    fn test_append_bytes_valid_utf8() {
        let mut buffer = String::new();
        append_bytes_to_buffer(&mut buffer, b"hello world");
        assert_eq!(buffer, "hello world");
    }

    #[test]
    fn test_append_bytes_invalid_utf8() {
        let mut buffer = String::new();
        append_bytes_to_buffer(&mut buffer, &[0xFF, 0xFE]);
        assert_eq!(buffer, "\u{FFFD}\u{FFFD}");
    }
}
