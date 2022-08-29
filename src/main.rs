async fn another_async() -> bool {
    async { true }.await
}

async fn one_async() -> bool {
    another_async().await
}

#[tokio::main]
async fn main() {
    if false {
        return Err(());
    }

    one_async()
}
