use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use eyre::Result;
use redis::Client;
use redis::Commands;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let provider_web_socket = check_env("PROVIDER_WSS", "wss://mainnet.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27".to_string());
    let redis_url = check_env("REDIS_URL", "redis://127.0.0.1/".to_string());

    let provider = Provider::<Ws>::connect(provider_web_socket).await?;
    let redis_client = Client::open(redis_url)?;

    let mut stream = provider.subscribe_blocks().await?;
    let mut redis_conn = redis_client.get_connection()?;
    let stream_name = "new_block".to_string();

    while let Some(block) = stream.next().await {
        let block_number = block.number.unwrap().to_string();
        let block_hash = block.hash.unwrap().to_string();
        println!(
            "Ts: {:?}, block number: {} -> {:?}",
            block.timestamp,
            block_number,
            block_hash,
        );

        redis_conn
            .xadd(
                &stream_name,
                "*",
                &[("block_number", block_number), ("block_hash", block_hash)],
            )?;

        redis_conn.xtrim(&stream_name, redis::streams::StreamMaxlen::Equals(1))?;
    }
    Ok(())
}

fn check_env (key: &str, default_value: String)-> String{
    match env::var(key) {
        Ok(val) => val,
        Err(e) => {
            println!("Error: {}", e);
            default_value
        },
    }
}