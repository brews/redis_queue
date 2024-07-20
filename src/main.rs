mod tasks;

use axum::extract::{Json, Path};
use axum::http::StatusCode;
use axum::routing::{post, Router};
use serde::{Deserialize, Serialize};
use std::env;
use std::time;

use tasks::{add::AddTask, multiply::MultiplyTask, subtract::SubtractTask, TaskMessage, TaskType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingBody {
    pub one: i32,
    pub two: i32,
}

async fn handler(
    Path(task_type): Path<String>,
    Json(body): Json<IncomingBody>,
) -> (StatusCode, String) {
    let client = redis::Client::open(env::var("REDIS_URL").expect("'REDIS_URL' must be set")).unwrap();
    let message_type: TaskType;
    match task_type.as_str() {
        "add" => {
            let body = AddTask {
                one: body.one,
                two: body.two,
            };
            message_type = TaskType::ADD(body);
        }
        "multiply" => {
            let body = MultiplyTask {
                one: body.one,
                two: body.two,
            };
            message_type = TaskType::MULTIPLY(body);
        }
        "subtract" => {
            let body = SubtractTask {
                one: body.one,
                two: body.two,
            };
            message_type = TaskType::SUBTRACT(body);
        }
        _ => return (StatusCode::NOT_FOUND, String::from("task not found")),
    }

    let message = TaskMessage { task: message_type };
    let serialized_message = bincode::serialize(&message).unwrap();
    let mut con = client.get_connection().unwrap();
    let _: () = redis::cmd("LPUSH")
        .arg("some_queue")
        .arg(serialized_message.clone())
        .query(&mut con)
        .unwrap();
    (StatusCode::CREATED, String::from("task sent"))
}

#[tokio::main]
async fn main() {
    let app_type = env::var("APP_TYPE").expect("APP_TYPE must be set");

    match app_type.as_str() {
        "server" => {
            // build our application with a single route
            let app = Router::new().route("/:task_type", post(handler));
            // run our app with hyper, listening globally on port 3000
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("server error: {}", e);
            }
        }
        "worker" => {
            let client = redis::Client::open(env::var("REDIS_URL").expect("'REDIS_URL' must be set")).unwrap();

            loop {
                let outcome: Option<Vec<u8>> = {
                    let mut con = client.get_connection().unwrap();
                    redis::cmd("RPOP")
                        .arg("some_queue")
                        .query(&mut con)
                        .unwrap()
                };
                match outcome {
                    Some(data) => {
                        let deserialized_message: TaskMessage =
                            bincode::deserialize(&data).unwrap();
                        match deserialized_message.task {
                            TaskType::ADD(task) => println!("{:?}", task.run()),
                            TaskType::MULTIPLY(task) => println!("{:?}", task.run()),
                            TaskType::SUBTRACT(task) => println!("{:?}", task.run()),
                        }
                    }
                    None => {
                        // Apparently no tasks. Just sleep.
                        let duration = time::Duration::from_secs(5);
                        tokio::time::sleep(duration).await;
                    }
                }
            }
        }
        _ => {
            panic!("{app_type} app type not supported");
        }
    }
}
