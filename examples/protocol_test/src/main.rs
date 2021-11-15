use std::time::Duration;

// use fe2o3_amqp::transport::connection::Connection;
// use fe2o3_amqp::transport::session::Session;

use fe2o3_amqp::connection::Connection;
use fe2o3_amqp::link::Sender;
use fe2o3_amqp::session::Session;

#[tokio::main]
async fn main() {
    println!("Starting connection");

    let mut connection = Connection::builder()
        .container_id("fe2o3-amqp")
        .hostname("127.0.0.1")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        .open("amqp://127.0.0.1:5674")
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let sender = Sender::attach(&mut session, "sender-link-1", "q1")
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
