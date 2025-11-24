use clap::Parser;
use futures::StreamExt;

#[allow(unused_imports)]
use log::{debug, info, warn, error};
use r2r::QosProfile;
use serde::{Deserialize, Serialize};
use std::{
    f64::consts::PI,
    net::{Ipv4Addr, SocketAddrV4}
};
use tokio::{sync::{OnceCell, RwLock}};
use warp::Filter;

static CLI_ARGS: OnceCell<CliArgs> = OnceCell::const_new();

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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    colog::init();

    let args = CliArgs::parse();
    CLI_ARGS.set(args.clone()).unwrap();

    info!("Configuration: {:#?}", args);

    info!("Starting ROS Node");
    // Set up ROS2 node inside the task
    let ctx = r2r::Context::create().unwrap();
    let mut node = r2r::Node::create(ctx.clone(), "web_server", "").unwrap();

    let subscription = node
        .subscribe::<r2r::sensor_msgs::msg::Imu>("/imu/imu", QosProfile::default())
        .unwrap();


    r2r::log_info!(node.logger(), "Initialized");
    info!("ROS Node Initialized");

    // Spawn a Tokio task (this is `Send` because it lives on the Tokio runtime)
    let join_handle = tokio::spawn(async move {
        subscription.for_each_concurrent(1, async |msg| {
            info!("value");
            // Convert quaternion z → angle (example, adjust as needed)
            let angle = msg.orientation.z;
            let angle_deg = angle / 180.0 * PI;
            let data = EventStructData {
                level: format!("Pitch = {}°", (args.target_angle - angle_deg)),
            };
            let mut guard = NUM.write().await;
            info!("got write guard");
            *guard = Some(data);
        }).await;
        error!("ROS Node Stopped");
    });

    node.spin_once(std::time::Duration::from_millis(100));

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
    tokio::spawn(warp::serve(route).run(CLI_ARGS.get().unwrap().socket));

    loop {
        node.spin_once(std::time::Duration::from_millis(100));        
    }
}
