use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use indicatif::ProgressBar;
use num_format::{Locale, ToFormattedString};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use solana_client::{
    nonblocking::rpc_client::RpcClient as AsyncRpcClient,
    rpc_client::{RpcClient, SerializableTransaction},
};
use solana_sdk::{instruction::Instruction, message::Message, pubkey::Pubkey};

/// Number of transactions to simulate
const TX_SIMS: u64 = 16;

/// Public solana mainnet beta endpoint
const MAINNET_BETA_ENDPOINT: &'static str = "https://api.mainnet-beta.solana.com";

#[tokio::main(worker_threads = 1)]
async fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .unwrap();

    // Initialize clients
    let sync_client = Arc::new(RpcClient::new(MAINNET_BETA_ENDPOINT));
    let async_client = Arc::new(AsyncRpcClient::new(MAINNET_BETA_ENDPOINT.to_string()));

    // Initialize progress bars
    let sync_pb = ProgressBar::new(TX_SIMS);
    let async_pb = ProgressBar::new(TX_SIMS);

    // Expected error
    // thread 'main' panicked at 'failed tx sim: ClientError { request: Some(SimulateTransaction),
    // kind: RpcError(RpcResponseError
    //    { code: -32602, message: "invalid transaction: Transaction failed to sanitize accounts offsets correctly", data: Empty }) }',
    //     src/main.rs:29:14
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    // Time synchronous simulations
    let sync_timer = Instant::now();
    (0..TX_SIMS)
        .into_par_iter()
        .for_each_with(sync_client, |client, _| {
            client
                .simulate_transaction(&transaction_builder())
                .expect_err("tx sim should fail");
            sync_pb.inc(1);
        });
    let sync_time = sync_timer.elapsed().as_micros();
    sync_pb.finish();

    println!("Sleeping to wait for mb rpc rate limits");
    std::thread::sleep(Duration::from_secs(20));

    // Time asynchronous simulations
    let async_timer = Instant::now();
    tokio_scoped::scope(|scope| {
        for _ in 0..TX_SIMS {
            let arc_client = Arc::clone(&async_client);
            let pb = async_pb.clone();
            scope.spawn(async move {
                arc_client
                    .simulate_transaction(&transaction_builder())
                    .await
                    .expect_err("tx sim should fail");
                pb.inc(1);
            });
        }
    });
    let async_time = async_timer.elapsed().as_micros();
    async_pb.finish();

    println!();
    println!("Results");
    println!(
        "    synchronous sims: {}",
        sync_time.to_formatted_string(&Locale::en)
    );
    println!(
        "   asynchronous sims: {}",
        async_time.to_formatted_string(&Locale::en)
    );
}

fn transaction_builder() -> impl SerializableTransaction {
    // Build an empty tx
    solana_sdk::transaction::Transaction::new_unsigned(Message::new(
        &[Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[],
            vec![],
        )],
        None,
    ))
}
