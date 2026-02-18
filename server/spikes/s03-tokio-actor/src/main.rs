use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;

const MAILBOX_CAPACITY: usize = 8;
const IDLE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
enum SessionCommand {
    ProcessMessage {
        input: String,
        reply: oneshot::Sender<String>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
struct SessionHandle {
    session_id: String,
    actor_id: u64,
    tx: mpsc::Sender<SessionCommand>,
}

impl SessionHandle {
    async fn process_message(&self, input: impl Into<String>) -> Result<String, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(SessionCommand::ProcessMessage {
                input: input.into(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| format!("session {} actor is closed", self.session_id))?;

        reply_rx
            .await
            .map_err(|_| format!("session {} reply channel dropped", self.session_id))
    }

    async fn shutdown(&self) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(SessionCommand::Shutdown { reply: reply_tx })
            .await
            .map_err(|_| format!("session {} actor is closed", self.session_id))?;

        reply_rx
            .await
            .map_err(|_| format!("session {} shutdown ack dropped", self.session_id))
    }
}

#[derive(Default)]
struct RegistryInner {
    actors: HashMap<String, SessionHandle>,
    next_actor_id: u64,
}

#[derive(Clone, Default)]
struct SessionRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

impl SessionRegistry {
    async fn get_or_create(&self, session_id: impl Into<String>) -> SessionHandle {
        let session_id = session_id.into();

        // Fast path: existing live actor.
        {
            let inner = self.inner.lock().await;
            if let Some(handle) = inner.actors.get(&session_id) {
                if !handle.tx.is_closed() {
                    return handle.clone();
                }
            }
        }

        // Slow path: create/recreate actor.
        let mut inner = self.inner.lock().await;
        if let Some(handle) = inner.actors.get(&session_id) {
            if !handle.tx.is_closed() {
                return handle.clone();
            }
        }

        inner.next_actor_id += 1;
        let actor_id = inner.next_actor_id;

        let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
        let handle = SessionHandle {
            session_id: session_id.clone(),
            actor_id,
            tx,
        };

        inner.actors.insert(session_id.clone(), handle.clone());
        drop(inner);

        let registry = self.clone();
        tokio::spawn(async move {
            run_session_actor(session_id.clone(), actor_id, rx).await;
            registry.remove_if_match(&session_id, actor_id).await;
        });

        handle
    }

    async fn remove_if_match(&self, session_id: &str, actor_id: u64) {
        let mut inner = self.inner.lock().await;
        let should_remove = inner
            .actors
            .get(session_id)
            .map(|h| h.actor_id == actor_id)
            .unwrap_or(false);
        if should_remove {
            inner.actors.remove(session_id);
        }
    }

    async fn actor_count(&self) -> usize {
        self.inner.lock().await.actors.len()
    }
}

async fn run_session_actor(
    session_id: String,
    actor_id: u64,
    mut rx: mpsc::Receiver<SessionCommand>,
) {
    println!("[actor {session_id}#{actor_id}] started");
    let mut turn: usize = 0;

    loop {
        match timeout(IDLE_TIMEOUT, rx.recv()).await {
            Ok(Some(SessionCommand::ProcessMessage { input, reply })) => {
                turn += 1;
                // Simulate long async work (e.g. LLM + store)
                tokio::time::sleep(Duration::from_millis(300)).await;
                let out = format!(
                    "session={session_id} turn={turn} processed input={input:?}"
                );
                let _ = reply.send(out);
            }
            Ok(Some(SessionCommand::Shutdown { reply })) => {
                let _ = reply.send(());
                println!("[actor {session_id}#{actor_id}] graceful shutdown");
                break;
            }
            Ok(None) => {
                println!("[actor {session_id}#{actor_id}] mailbox closed");
                break;
            }
            Err(_) => {
                println!("[actor {session_id}#{actor_id}] idle timeout, stopping");
                break;
            }
        }
    }

    println!("[actor {session_id}#{actor_id}] exited");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SessionRegistry::default();

    // Same session: concurrent callers, single writer serialization in actor.
    let s1 = registry.get_or_create("session-A").await;
    let s2 = s1.clone();
    let s3 = s1.clone();

    let t1 = tokio::spawn(async move { s1.process_message("hello").await });
    let t2 = tokio::spawn(async move { s2.process_message("from").await });
    let t3 = tokio::spawn(async move { s3.process_message("tokio actor").await });

    println!("{}", t1.await??);
    println!("{}", t2.await??);
    println!("{}", t3.await??);

    // Different session: independent actor, can run concurrently.
    let a = registry.get_or_create("session-B").await;
    println!("{}", a.process_message("another session").await?);

    println!("active actors(before shutdown)={}", registry.actor_count().await);

    // Graceful shutdown demo.
    let a2 = registry.get_or_create("session-B").await;
    a2.shutdown().await?;

    // Wait for idle timeout cleanup on session-A.
    tokio::time::sleep(Duration::from_secs(4)).await;

    println!("active actors(after cleanup)={}", registry.actor_count().await);
    Ok(())
}
