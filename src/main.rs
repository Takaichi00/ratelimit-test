use anyhow::Result;
use futures::future::try_join_all;
use governor::{Quota, RateLimiter};
use std::{num::NonZeroU32, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let samples: Vec<Sample> = (1..=10000)
        .map(|i| Sample {
            name: format!("Sample {}", i),
        })
        .collect();

    // レートリミッターの設定: 1分あたり240回のリクエストを許可し、バースト (初期可能リクエスト) は1回
    let quota =
        Quota::per_minute(NonZeroU32::new(240).unwrap()).allow_burst(NonZeroU32::new(1).unwrap());

    // Arc で包んで複数スレッドから参照可能にする
    let limiter = Arc::new(RateLimiter::direct(quota));

    // samples を4分割して並列処理する
    let chunk_size = (samples.len() + 3) / 4;
    let mut handles = Vec::with_capacity(4);
    for chunk in samples.chunks(chunk_size) {
        let limiter = limiter.clone();
        // チャンクを Vec<Sample> に変換
        let chunk = chunk.iter().cloned().collect::<Vec<Sample>>();
        handles.push(tokio::spawn(async move {
            for sample in chunk {
                limiter.until_ready().await;
                rate_limited_task(sample.name).await?;
            }
            Ok::<(), anyhow::Error>(())
        }));
    }

    let _result: Vec<()> = try_join_all(handles)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    println!("All tasks completed.");

    Ok(())
}

#[derive(Clone, Debug)]
pub struct Sample {
    pub name: String,
}

async fn rate_limited_task(name: String) -> Result<()> {
    let thread_id = std::thread::current().id();
    println!("[thread {:?}] Task executed: {}", thread_id, name);
    Ok(())
}
