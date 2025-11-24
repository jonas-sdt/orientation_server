use clap::Parser;
use futures::{StreamExt, executor::LocalPool, task::LocalSpawnExt};

#[allow(unused_imports)]
use log::{debug, info};
use r2r::QosProfile;
use serde::{Deserialize, Serialize};
use std::{
    f64::consts::PI,
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::sync::OnceCell;
use warp::Filter;

static CLI_ARGS: OnceCell<CliArgs> = OnceCell::const_new();
static LOGGER_NAME: OnceCell<String> = OnceCell::const_new();
// static NODE: OnceCell<&Node> = OnceCell::const_new();

#[derive(clap::Parser, Debug, PartialEq, Clone)]
struct CliArgs {
    #[arg(short, long, default_value_t = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3030))]
    socket: SocketAddrV4,
    #[arg(short, long, default_value_t = 50)]
    update_intervall_ms: u64,
    #[arg(short, long, default_value_t = 80.0)]
    target_angle: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct EventStructData {
    level: String,
}

#[tokio::main]
async fn main() {
    colog::init();

    let args = CliArgs::parse();
    CLI_ARGS.set(args.clone()).unwrap();

    info!("Configuration: {:#?}", args);

    let cors = warp::cors()
        .allow_any_origin() // Allow all origins
        .allow_methods(vec!["GET"]) // Allow GET requests
        .allow_headers(vec!["Content-Type"]); // Allow 

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let sse_route = warp::path("events").and(warp::get()).map(|| {
        let ctx = r2r::Context::create().unwrap();
        let mut node = r2r::Node::create(ctx.clone(), "web_server", "").unwrap();
        LOGGER_NAME.set(node.logger().to_string()).unwrap();

        let mut subscription = node
            .subscribe::<r2r::sensor_msgs::msg::Imu>("/imu/imu", QosProfile::default())
            .unwrap();

        r2r::log_info!(node.logger(), "Intialized");

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        spawner
            .spawn_local(async move {
                while let Some(msg) = subscription.next().await {
                    let angle = msg.orientation.z;
                    let angle_deg = angle / 180.0 * PI;
                    let data = EventStructData {
                        level: format!("Pitch = {}°", (args.target_angle - angle_deg)),
                    };
                    let event = warp::sse::Event::default().json_data(data);

                    tx.send(event).await.unwrap();
                }
            })
            .unwrap();

        // Convert receiver to a stream
        let stream = async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                yield msg; // Yield messages to the stream
            }
        };
        warp::sse::reply(warp::sse::keep_alive().stream(stream))
    });

    let route = sse_route.with(cors);

    // Start the warp server
    warp::serve(route)
        .run(CLI_ARGS.get().unwrap().socket) // Listen on localhost:3030
        .await;
}
