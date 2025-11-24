use clap::Parser;
use futures::{Stream, StreamExt, executor::LocalPool, task::LocalSpawnExt};
use log::{debug, info};
use r2r::{Node, QosProfile, log_info};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    f64::consts::PI,
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::{
    sync::OnceCell,
    time::{self, Duration},
};
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

// Function to create a stream of events
async fn event_stream() -> Result<impl warp::Reply, Infallible> {
    // let (tx, mut rx) = tokio::sync::mpsc::channel::<EventStructData>(32);

    let ctx = r2r::Context::create().unwrap();
    let mut node = r2r::Node::create(ctx.clone(), "web_server", "").unwrap();
    LOGGER_NAME.set(node.logger().to_string()).unwrap();

    let mut subscription = node
        .subscribe::<r2r::sensor_msgs::msg::Imu>("/imu/imu", QosProfile::default())
        .unwrap();

    r2r::log_info!(node.logger(), "Intialized");

    info!("Created Subscriber");
    let args = CLI_ARGS.get().unwrap();
    info!("Starting with Config: {:#?}", args);

    node.spin_once(Duration::from_secs(1));

    let stream = subscription.map(|msg| {
        let args = CLI_ARGS.get().unwrap();
        let level = msg.orientation.y;
        let level_deg = level / 180.0 * PI;
        let data = EventStructData {
            level: format!("Pitch = {}°", (args.target_angle - level_deg)),
        };
        let event = warp::sse::Event::default().json_data(data);
        event
    });

    // Create a stream of SSE
    // let stream = async_stream::stream! {
    //     info!("Starting loop");
    //     // let node = node;
    //     let mut subscription = subscription;
    //     while ctx.is_valid() {
    //         while let Some(msg) = subscription.next().await {
    //             let args = CLI_ARGS.get().unwrap();
    //             let level = msg.orientation.y;
    //             let level_deg = level / 180.0 * PI;
    //             let data = EventStructData {level: format!("Pitch = {}°", (args.target_angle - level_deg))};
    //             let event = warp::sse::Event::default().json_data(data);
    //             yield event;
    //         }
    //         tokio::time::sleep(Duration::from_millis(50)).await;
    //     }
    // info!("Stopping loop");
    // };
    Ok(warp::sse::reply(warp::sse::keep_alive().stream(stream)))
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

    // Define a route for SSE
    let route = warp::path("events")
        .and(warp::get())
        .and_then(event_stream)
        .with(cors);

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    spawner
        .spawn_local(async move {
            // Start the warp server
            warp::serve(route)
                .run(CLI_ARGS.get().unwrap().socket) // Listen on localhost:3030
                .await;
        })
        .unwrap();

    spawner.spawn_local(async move {}).unwrap();
}
