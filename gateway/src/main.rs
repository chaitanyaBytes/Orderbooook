use runtime::RUNTIME;

fn main() {
    RUNTIME.block_on(async {
        println!("Hello from global runtime");

        let handle = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            "done"
        });

        let result = handle.await.unwrap();
        println!("Task result: {result}");
    });
}
