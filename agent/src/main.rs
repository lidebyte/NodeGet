#![feature(duration_millis_float)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::await_holding_lock,
    dead_code
)]

use crate::tasks::ping::http::httping_target;
use crate::tasks::ping::icmp::ping_v4_target;
use crate::tasks::ping::tcp::tcping_target;
use std::net::SocketAddr;

mod monitoring;
mod tasks;

#[tokio::main]
async fn main() {
    println!(
        "{}",
        ping_v4_target("1.1.1.1".parse().unwrap())
            .await
            .unwrap()
            .as_millis_f64()
    );

    println!(
        "{}",
        tcping_target(SocketAddr::new("1.1.1.1".parse().unwrap(), 80))
            .await
            .unwrap()
            .as_millis_f64()
    );

    println!(
        "{}",
        httping_target("https://1.1.1.1")
            .await
            .unwrap()
            .as_millis_f64()
    );
}
