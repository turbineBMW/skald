//! Tokio runtime for network I/O; results are awaited from the GLib main loop
//! (`tokio::task::JoinHandle` is executor-agnostic).
use std::sync::OnceLock;
pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio"))
}
/// Spawn `fut` on tokio and await it from a GLib-local context.
pub async fn io<T: Send + 'static>(fut: impl std::future::Future<Output = T> + Send + 'static) -> T {
    rt().spawn(fut).await.expect("tokio task panicked")
}
