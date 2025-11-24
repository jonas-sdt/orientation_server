use clap::Parser;
use futures::StreamExt;

#[allow(unused_imports)]
use log::{debug, info};
use r2r::QosProfile;
use serde::{Deserialize, Serialize};
use std::{
    f64::consts::PI,
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::sync::{OnceCell, RwLock};
use warp::Filter;

static CLI_ARGS: OnceCell<CliArgs> = OnceCell::const_new();
static LOGGER_NAME: OnceCell<String> = OnceCell::const_new();

static NUM: RwLock<Option<EventStructData>> = RwLock::const_new(None);

#[derive(clap::Parser, Debug, PartialEq, Clone)]
struct CliArgs {
    #[arg(short, long, default_value_t = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3030))]
    socket: SocketAddrV4,
    #[arg(short, long, default_value_t = 50)]
    update_intervall_ms: u64,
    #[arg(short, long, default_value_t = 80.0)]
    target_angle: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EventStructData {
    level: String,
}

#[tokio::main]
async fn main() {
    colog::init();

    let args = CliArgs::parse();
    CLI_ARGS.set(args.clone()).unwrap();

    info!("Configuration: {:#?}", args);

    // Spawn a Tokio task (this is `Send` because it lives on the Tokio runtime)
    tokio::spawn(async move {
        // Set up ROS2 node inside the task
        let ctx = r2r::Context::create().unwrap();
        let mut node = r2r::Node::create(ctx.clone(), "web_server", "").unwrap();
        LOGGER_NAME.set(node.logger().to_string()).unwrap();

        let mut subscription = node
            .subscribe::<r2r::sensor_msgs::msg::Imu>("/imu/imu", QosProfile::default())
            .unwrap();

        r2r::log_info!(node.logger(), "Initialized");

        while let Some(msg) = subscription.next().await {
            // Convert quaternion z → angle (example, adjust as needed)
            let angle = msg.orientation.z;
            let angle_deg = angle / 180.0 * PI;
            let data = EventStructData {
                level: format!("Pitch = {}°", (args.target_angle - angle_deg)),
            };
            let mut guard = NUM.write().await;
            *guard = Some(data);
        }
    });

    // ---------- CORS ----------
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET"])
        .allow_headers(vec!["Content-Type"]);

    // ---------- SSE route ----------
    let sse_route = warp::path("events").and(warp::get()).map(move || {
        info!("new connection");

        // Turn the receiver into a stream that warp can use
        let stream = async_stream::stream! {
            if let Some(event_data) = NUM.read().await.clone() {
                let event = warp::sse::Event::default().json_data(event_data);
                yield event;
            } else {
                let event = warp::sse::Event::default().json_data(EventStructData { level: "no data".to_string() });
                yield event;
                
            }
        };

        warp::sse::reply(warp::sse::keep_alive().stream(stream))
    });

    // ---------- Combine route & CORS ----------
    let route = sse_route.with(cors);

    // ---------- Run the server ----------
    warp::serve(route).run(CLI_ARGS.get().unwrap().socket).await;
}
