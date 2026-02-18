use std::sync::Arc;

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};

#[derive(Debug, Clone)]
pub struct AgentEventEnvelope {
    pub topic: String,
}

#[derive(Debug, Clone)]
pub struct SubscriptionRequest {
    pub topic: String,
}

#[derive(Debug, Clone)]
pub struct FanoutPolicy;

#[derive(Debug, Clone)]
pub struct DeliveryReport {
    pub accepted: usize,
}

#[derive(Debug, Clone)]
pub struct ChannelError(pub String);

pub type EventItem = Result<AgentEventEnvelope, ChannelError>;

// =========================
// 方案 A：去掉关联类型，直接返回 Box<dyn Stream>
// =========================

pub type DynEventStream = Box<dyn Stream<Item = EventItem> + Send + Unpin + 'static>;

#[async_trait]
pub trait ChannelAdapterDyn: Send + Sync {
    async fn subscribe(&self, req: SubscriptionRequest) -> Result<DynEventStream, ChannelError>;

    async fn publish(
        &self,
        envelope: AgentEventEnvelope,
        policy: FanoutPolicy,
    ) -> Result<DeliveryReport, ChannelError>;
}

pub struct FakeDynAdapter;

#[async_trait]
impl ChannelAdapterDyn for FakeDynAdapter {
    async fn subscribe(&self, req: SubscriptionRequest) -> Result<DynEventStream, ChannelError> {
        let s = stream::iter(vec![Ok(AgentEventEnvelope {
            topic: req.topic,
        })]);
        Ok(Box::new(s))
    }

    async fn publish(
        &self,
        _envelope: AgentEventEnvelope,
        _policy: FanoutPolicy,
    ) -> Result<DeliveryReport, ChannelError> {
        Ok(DeliveryReport { accepted: 1 })
    }
}

pub struct AppServiceDyn {
    adapter: Arc<dyn ChannelAdapterDyn + Send + Sync>,
}

impl AppServiceDyn {
    pub fn new(adapter: Arc<dyn ChannelAdapterDyn + Send + Sync>) -> Self {
        Self { adapter }
    }

    pub async fn one_event_topic(&self, topic: &str) -> Result<Option<String>, ChannelError> {
        let mut stream = self
            .adapter
            .subscribe(SubscriptionRequest {
                topic: topic.to_string(),
            })
            .await?;

        let next = stream.next().await.transpose()?;
        Ok(next.map(|e| e.topic))
    }
}

// =========================
// 方案 B：保留关联类型，使用泛型持有 adapter
// =========================

#[async_trait]
pub trait ChannelAdapterGeneric: Send + Sync {
    type EventStream: Stream<Item = EventItem> + Send + Unpin + 'static;

    async fn subscribe(&self, req: SubscriptionRequest) -> Result<Self::EventStream, ChannelError>;

    async fn publish(
        &self,
        envelope: AgentEventEnvelope,
        policy: FanoutPolicy,
    ) -> Result<DeliveryReport, ChannelError>;
}

pub struct FakeGenericAdapter;

#[async_trait]
impl ChannelAdapterGeneric for FakeGenericAdapter {
    type EventStream = stream::Iter<std::vec::IntoIter<EventItem>>;

    async fn subscribe(&self, req: SubscriptionRequest) -> Result<Self::EventStream, ChannelError> {
        Ok(stream::iter(vec![Ok(AgentEventEnvelope {
            topic: req.topic,
        })]))
    }

    async fn publish(
        &self,
        _envelope: AgentEventEnvelope,
        _policy: FanoutPolicy,
    ) -> Result<DeliveryReport, ChannelError> {
        Ok(DeliveryReport { accepted: 1 })
    }
}

pub struct AppServiceGeneric<A>
where
    A: ChannelAdapterGeneric + Send + Sync + 'static,
{
    adapter: Arc<A>,
}

impl<A> AppServiceGeneric<A>
where
    A: ChannelAdapterGeneric + Send + Sync + 'static,
{
    pub fn new(adapter: Arc<A>) -> Self {
        Self { adapter }
    }

    pub async fn one_event_topic(&self, topic: &str) -> Result<Option<String>, ChannelError> {
        let mut stream = self
            .adapter
            .subscribe(SubscriptionRequest {
                topic: topic.to_string(),
            })
            .await?;

        let next = stream.next().await.transpose()?;
        Ok(next.map(|e| e.topic))
    }
}

// 这个函数若取消注释，会展示“关联类型 trait 不能直接用 dyn”这一限制：
// pub fn not_object_safe_example(_a: Arc<dyn ChannelAdapterGeneric + Send + Sync>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn dyn_adapter_works() {
        let svc = AppServiceDyn::new(Arc::new(FakeDynAdapter));
        let topic = svc.one_event_topic("grpc").await.unwrap();
        assert_eq!(topic.as_deref(), Some("grpc"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn generic_adapter_works() {
        let svc = AppServiceGeneric::new(Arc::new(FakeGenericAdapter));
        let topic = svc.one_event_topic("ws").await.unwrap();
        assert_eq!(topic.as_deref(), Some("ws"));
    }
}
