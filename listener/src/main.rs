use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use eyre::Result;
use redis::Client;
use redis::Commands;
use std::env;
use ethers::prelude::H256;
use ethers::types::Block;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let provider_web_socket = env::var("PROVIDER_WSS").unwrap_or("wss://mainnet.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27".to_string());
    let redis_host = env::var("REDIS_HOST").unwrap_or("redis://127.0.0.1".to_string());

    let provider = Provider::<Ws>::connect(provider_web_socket).await?;
    let redis_client = Client::open(redis_host)?;

    let (tx, mut rx): (mpsc::Sender<Block<H256>>, mpsc::Receiver<Block<H256>>) = mpsc::channel(32);

    let provider_task1 = tokio::spawn(async move {
        let mut stream = provider.subscribe_blocks().await?;
        while let Some(block) = stream.next().await {
            tx.send(block).await?;
        }
        Result::<()>::Ok(())
    });

    let mut redis_conn = redis_client.get_connection()?;
    let stream_name = "new_block".to_string();

    while let Some(block) = rx.recv().await {
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
                &[("block_number", block_number), ("block_hash", block_hash), ("block_info", serde_json::to_string(&block).unwrap())],
            )?;

        redis_conn.xtrim(&stream_name, redis::streams::StreamMaxlen::Equals(1))?;
    }

    provider_task1.await??;

    Ok(())
}