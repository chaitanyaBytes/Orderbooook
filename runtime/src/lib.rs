use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("io-runtime")
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
});

#[cfg(test)]
mod tests {
    use super::RUNTIME;

    #[test]
    fn runtime_spawns_and_runs_tasks() {
        RUNTIME.block_on(async {
            let handle = tokio::spawn(async { 1 + 1 });
            let result = handle.await.unwrap();
            assert_eq!(result, 2);
        });
    }
}
