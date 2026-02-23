use std::sync::{Arc, Mutex};

use agent_domain::{LlmError, LlmProvider, LlmRequest, LlmResponse};

#[derive(Clone)]
pub struct MockLlmProvider {
    response: Arc<Mutex<Result<LlmResponse, LlmError>>>,
    calls: Arc<Mutex<Vec<LlmRequest>>>,
}

impl MockLlmProvider {
    pub fn new(response: Result<LlmResponse, LlmError>) -> Self {
        Self {
            response: Arc::new(Mutex::new(response)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 创建一个返回固定文本的 mock
    pub fn with_text(text: &str) -> Self {
        Self::new(Ok(LlmResponse {
            content: text.to_string(),
            model: "mock".to_string(),
            usage: None,
        }))
    }

    /// 获取调用记录
    pub fn calls(&self) -> Vec<LlmRequest> {
        self.calls
            .lock()
            .expect("calls mutex should not be poisoned")
            .clone()
    }

    /// 动态更新返回值
    pub fn set_response(&self, response: Result<LlmResponse, LlmError>) {
        *self
            .response
            .lock()
            .expect("response mutex should not be poisoned") = response;
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.calls
            .lock()
            .expect("calls mutex should not be poisoned")
            .push(request);
        self.response
            .lock()
            .expect("response mutex should not be poisoned")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    use agent_domain::{ChatMessage, LlmError, LlmProvider, LlmRequest};

    use super::MockLlmProvider;

    #[test]
    fn generate_returns_text_and_records_call() {
        let provider = MockLlmProvider::with_text("hello");
        let request = LlmRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            model: Some("test-model".to_string()),
            temperature: Some(0.3),
            max_tokens: Some(10),
        };

        let response = block_on(provider.generate(request.clone())).expect("mock should return ok");

        assert_eq!(response.content, "hello");
        assert_eq!(provider.calls(), vec![request]);
    }

    #[test]
    fn generate_returns_error() {
        let provider = MockLlmProvider::new(Err(LlmError::Timeout));
        let request = LlmRequest {
            messages: vec![],
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let result = block_on(provider.generate(request.clone()));

        assert_eq!(result, Err(LlmError::Timeout));
        assert_eq!(provider.calls(), vec![request]);
    }

    fn block_on<F: Future>(mut future: F) -> F::Output {
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        // SAFETY: future is pinned and not moved afterwards.
        let mut future = unsafe { Pin::new_unchecked(&mut future) };

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn noop_waker() -> Waker {
        // SAFETY: no-op raw waker functions are valid for a stateless waker.
        unsafe { Waker::from_raw(noop_raw_waker()) }
    }

    fn noop_raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}

        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
        )
    }
}
