use std::{io, thread};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle};
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{Data};
use futures::future::join_all;
use serde_json::json;
use crate::consensus::message::RequestMsg;
use crate::network::node::Node;
use crate::network::utils::compute_digest;

#[derive(Clone)]
pub(crate) struct Server {
    port: u16,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub node: Node,
}

impl Server {
    pub(crate) fn new(node_id: u32, port: u16, n: u32, is_faulty: bool) -> Self {
        Self {
            port,
            handle: Arc::new(Mutex::new(None)),
            node: Node::new(node_id, n, is_faulty),
        }
    }

    pub(crate) fn start(&mut self) {
        let port = self.port;
        let server_data = Arc::new(self.clone());
        let server_data_clone = server_data.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = start_server(port, server_data_clone) {
                eprintln!("Server failed to start on port {}: {}", port, e);
            }
        });
        let mut handle_lock = self.handle.lock().unwrap();
        *handle_lock = Some(handle);
    }

    pub(crate) fn join(self) {
        // try to acquire lock on handle
        if let Ok(mut handle_lock) = self.handle.lock() {
            // use `take()` method to take out the value in `Option<JoinHandle<()>>` and set it to `None`
            if let Some(handle) = handle_lock.take() {
                handle.join().unwrap();
            }
        } else {
            eprintln!("Failed to acquire lock on handle");
        }
    }
}

#[post("/req")]
async fn handle_req(request_msg: web::Json<RequestMsg>, server_data: Data<Server>) -> impl Responder {
    let response_body = json!({"status": "ok"});
    let mut emoji = "😃";
    if server_data.node.is_faulty {
        emoji = "😈";
    }
    println!("[{} Node{}] Received RequestMsg: {:?}", emoji, server_data.node.id, request_msg);
    // Paper 4.3, second paragraph
    // When the primary receives a client request,
    // it assigns a sequence number to the request
    // and multicasts a pre-prepare message for that sequence number.
    let mut request_msg = request_msg.into_inner();
    request_msg.digest = compute_digest(&request_msg);
    // println!("digest: {}", request_msg.digest);
    server_data.node.msg_buffer.request_msgs.lock().unwrap().push(request_msg.clone());
    {
        let mut current_stage_guard = server_data
            .node
            .current_state
            .current_stage
            .lock()
            .unwrap();
        if *current_stage_guard == crate::consensus::pbft::Stage::Idle {
            *current_stage_guard = crate::consensus::pbft::Stage::PrePrepare;
        }
    }
    println!("🌟[{} Node{}] Transitioned to PrePrepare stage!", emoji, server_data.node.id);
    if server_data.node.id == server_data.node.view.primary_node_id {
        // primary node
        request_msg.sequence_id = 0; // TODO: assign sequence id
        let digest  = request_msg.digest.clone();
        let pre_prepare_msg = crate::consensus::message::PrePrepareMsg {
            view_id: server_data.node.view.id,
            sequence_id: request_msg.sequence_id,
            digest,
            request_msg,
        };
        // send pre-prepare message to all nodes
        let client = reqwest::Client::new();
        let requests: Vec<_> = server_data.node.node_table.iter().filter_map(|(id, url)| {
            if *id == server_data.node.id {
                return None; // Skip self
            }

            let pre_prepare_msg_clone = pre_prepare_msg.clone();
            let client_clone = client.clone();
            let url_clone = url.clone();
            let id_clone = *id;
            let emoji_clone = emoji;
            let server_data_clone = server_data.clone();

            // Return a future representing the request
            Some(async move {
                println!(
                    "[{} Node{}] Sending PrePrepareMsg to node {}: {:?}",
                    emoji_clone, server_data_clone.node.id, id_clone, pre_prepare_msg_clone
                );

                match client_clone
                    .post(&format!("http://{}/preprepare", url_clone))
                    .json(&pre_prepare_msg_clone)
                    .send()
                    .await
                {
                    Ok(response) => {
                        println!("  -- Response from server {}: {:?}", id_clone, response.status());
                    }
                    Err(e) => {
                        eprintln!(" -- Error sending request to node {}: {}", id_clone, e);
                    }
                }
            })
        }).collect();
        {
            let mut current_stage_guard = server_data
                .node
                .current_state
                .current_stage
                .lock()
                .unwrap();
            if *current_stage_guard == crate::consensus::pbft::Stage::PrePrepare {
                *current_stage_guard = crate::consensus::pbft::Stage::Prepare;
            }
        }
        println!("🌟🌟[{} Primary Node{}] Transitioned to Prepare stage!", emoji, server_data.node.id);
        // Run all requests concurrently
        join_all(requests).await;
        // set primary node to the next stage
    }
    HttpResponse::Ok().json(response_body)
}
#[post("/preprepare")]
async fn handle_pre_prepare(pre_prepare_msg: web::Json<crate::consensus::message::PrePrepareMsg>, server_data: Data<Server>) -> impl Responder {
    let response_body = json!({"status": "ok"});
    let mut emoji = "😃";
    if server_data.node.is_faulty {
        emoji = "😈";
    }
    println!("[{} Node{}] Received PrePrepareMsg: {:?}", emoji, server_data.node.id, pre_prepare_msg);
    let mut request_msg = RequestMsg { // request message corresponding to the pre-prepare message
        time_stamp: 0,
        client_id: 0,
        operation: String::new(),
        sequence_id: 0,
        digest: "".to_string(),
    };
    let digest = pre_prepare_msg.digest.clone();
    for msg in server_data.node.msg_buffer.request_msgs.lock().unwrap().iter() {
        if digest == msg.digest {
            request_msg = msg.clone();
            break;
        }
    }
    if request_msg.digest == "" {
        eprintln!("[{} Node{}] Request message not found for PrePrepareMsg", emoji, server_data.node.id);
        return HttpResponse::Ok().json(response_body);
    }
    let verify_result = crate::network::utils::verify_msg(&pre_prepare_msg, server_data.node.view.id, request_msg.digest.clone());
    if !verify_result {
        eprintln!("[{} Node{}] PrePrepareMsg verification failed", emoji, server_data.node.id);
        HttpResponse::Ok().json(response_body)
    } else {
        let view_id = pre_prepare_msg.view_id;
        let sequence_id = pre_prepare_msg.sequence_id;
        let digest = pre_prepare_msg.digest.clone();
        server_data.node.msg_buffer.preprepare_msgs.lock().unwrap().push(pre_prepare_msg.into_inner());
        // If node i is accepting the pre-prepare message, it transitions to the Prepare stage
        // by multicasting a prepare message to all other nodes
        {
            let mut current_stage_guard = server_data
                .node
                .current_state
                .current_stage
                .lock()
                .unwrap();
            if *current_stage_guard == crate::consensus::pbft::Stage::PrePrepare {
                *current_stage_guard = crate::consensus::pbft::Stage::Prepare;
            }
        }
        println!("🌟🌟[{} Node{}] Transitioned to Prepare stage!", emoji, server_data.node.id);

        let prepare_msg = crate::consensus::message::VoteMsg {
            view_id,
            sequence_id,
            digest,
            node_id: server_data.node.id,
            msg_type: crate::consensus::message::MsgType::PrepareMsg,
        };

        server_data.node.msg_buffer.prepare_msgs.lock().unwrap().push(prepare_msg.clone()); // save the prepare message
        let client = reqwest::Client::new();
        let requests: Vec<_> = server_data.node.node_table.iter().filter_map(|(id, url)| {
            if *id == server_data.node.id {
                return None; // Skip self
            }

            let prepare_msg_clone = prepare_msg.clone();
            let client_clone = client.clone();
            let url_clone = url.clone();
            let id_clone = *id;
            let emoji_clone = emoji;
            let server_data_clone = server_data.clone();
            // Return a future representing the request
            Some(async move {
                println!(
                    "[{} Node{}] Sending PrepareMsg to node {}: {:?}",
                    emoji_clone, server_data_clone.node.id, id_clone, prepare_msg_clone
                );
                match client_clone.post(&format!("http://{}/prepare", url_clone))
                    .json(&prepare_msg_clone)
                    .send()
                    .await
                {
                    Ok(response) => {
                        println!("  -- Response from server {}: {:?}", id_clone, response.status());
                    }
                    Err(e) => {
                        eprintln!(" -- Error sending request to node {}: {}", id_clone, e);
                    }
                }
            })
        }).collect();

        // Run all requests concurrently
        join_all(requests).await;
        HttpResponse::Ok().json(response_body)
    }
}

#[post("/prepare")]
async fn handle_prepare(prepare_msg: web::Json<crate::consensus::message::VoteMsg>, server_data: Data<Server>) -> impl Responder {
    let response_body = json!({"status": "ok"});
    let mut emoji = "😃";
    if server_data.node.is_faulty {
        emoji = "😈";
    }
    println!("[{} Node{}] Received PrepareMsg: {:?}", emoji, server_data.node.id, prepare_msg);
    let n = server_data.node.node_table.len();
    let f = (n - 1) / 3;
    let prepare_msg = prepare_msg.into_inner();
    let mut preprepare_msg = crate::consensus::message::PrePrepareMsg {
        view_id: prepare_msg.view_id,
        sequence_id: prepare_msg.sequence_id,
        digest: prepare_msg.digest.clone(),
        request_msg: RequestMsg {
            time_stamp: 0,
            client_id: 0,
            operation: String::new(),
            sequence_id: 0,
            digest: "".to_string(),
        },
    };
    for msg in server_data.node.msg_buffer.preprepare_msgs.lock().unwrap().iter() {
        if msg.digest == prepare_msg.digest && msg.view_id == prepare_msg.view_id {
            preprepare_msg = msg.clone();
            break;
        }
    }
    if preprepare_msg.digest == "" {
        eprintln!("[{} Node{}] PrePrepareMsg not found for PrepareMsg", emoji, server_data.node.id);
        return HttpResponse::Ok().json(response_body);
    } else {
        server_data.node.msg_buffer.prepare_msgs.lock().unwrap().push(prepare_msg.clone()); // save the prepare message
        let mut cnt = 0;
        for msg in server_data.node.msg_buffer.prepare_msgs.lock().unwrap().iter() {
            if msg.digest == prepare_msg.digest && msg.view_id == prepare_msg.view_id {
                cnt += 1;
            }
        }
        if cnt == 2 * f + 1 {
            {
                let mut current_stage_guard = server_data
                    .node
                    .current_state
                    .current_stage
                    .lock()
                    .unwrap();
                if *current_stage_guard == crate::consensus::pbft::Stage::Prepare {
                    *current_stage_guard = crate::consensus::pbft::Stage::Commit;
                }
            }
            println!(
                "🌟🌟🌟[{} Node{}] Transitioned to Commit stage!",
                emoji, server_data.node.id
            );
            let commit_msg = crate::consensus::message::VoteMsg {
                view_id: prepare_msg.view_id,
                sequence_id: prepare_msg.sequence_id,
                digest: prepare_msg.digest.clone(),
                node_id: server_data.node.id,
                msg_type: crate::consensus::message::MsgType::CommitMsg,
            };
            server_data.node.msg_buffer.commit_msgs.lock().unwrap().push(commit_msg.clone());
            let client = reqwest::Client::new();
            let requests: Vec<_> = server_data.node.node_table.iter().filter_map(|(id, url)| {
                if *id == server_data.node.id {
                    return None; // Skip self
                }

                let commit_msg_clone = commit_msg.clone();
                let client_clone = client.clone();
                let url_clone = url.clone();
                let id_clone = *id;
                let emoji_clone = emoji;
                let server_data_clone = server_data.clone();
                // Return a future representing the request
                Some(async move {
                    println!(
                        "[{} Node{}] Sending CommitMsg to node {}: {:?}",
                        emoji_clone, server_data_clone.node.id, id_clone, commit_msg_clone
                    );
                    match client_clone.post(&format!("http://{}/commit", url_clone))
                        .json(&commit_msg_clone)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            println!("  -- Response from server {}: {:?}", id_clone, response.status());
                        }
                        Err(e) => {
                            eprintln!(" -- Error sending request to node {}: {}", id_clone, e);
                        }
                    }
                })
            }).collect();
            // Run all requests concurrently
            join_all(requests).await;
        }
    }
    HttpResponse::Ok().json(response_body)
}

#[post("/commit")]
async fn handle_commit(commit_msg: web::Json<crate::consensus::message::VoteMsg>, server_data: Data<Server>) -> impl Responder {
    let response_body = json!({"status": "ok"});
    let mut emoji = "😃";
    if server_data.node.is_faulty {
        emoji = "😈";
    }
    println!("[{} Node{}] Received CommitMsg: {:?}", emoji, server_data.node.id, commit_msg);
    let n = server_data.node.node_table.len();
    let f = (n - 1) / 3;
    let commit_msg = commit_msg.into_inner();
    server_data.node.msg_buffer.commit_msgs.lock().unwrap().push(commit_msg.clone());
    let mut cnt = 0;
    for msg in server_data.node.msg_buffer.commit_msgs.lock().unwrap().iter() {
        if msg.digest == commit_msg.digest && msg.view_id == commit_msg.view_id {
            cnt += 1;
        }
    }
    if cnt == 2 * f + 1 {
        // println!("[{} Node{}] Request committed!", emoji, server_data.node.id);
        let mut request_msg = RequestMsg {
            time_stamp: 0,
            client_id: 0,
            operation: String::new(),
            sequence_id: 0,
            digest: "".to_string(),
        };
        for msg in server_data.node.msg_buffer.request_msgs.lock().unwrap().iter() {
            if msg.digest == commit_msg.digest {
                request_msg = msg.clone();
                break;
            }
        }
        if request_msg.digest != "" {
            let client = reqwest::Client::new();
            let reply_msg = crate::consensus::message::ReplyMsg {
                time_stamp: request_msg.time_stamp,
                view_id: commit_msg.view_id,
                node_id: server_data.node.id,
                client_id: request_msg.client_id,
                result: request_msg.operation.clone(),
            };
            // post request to 127.0.0.1:9000
            match client.post("http://127.0.0.1:9000/reply")
                .json(&reply_msg)
                .send()
                .await
            {
                Ok(response) => {
                    println!("[{} Node{}] Response from client server: {:?}", emoji, server_data.node.id, response.status());
                }
                Err(e) => {
                    eprintln!("[{} Node{}] Error sending request to client server: {}", emoji, server_data.node.id, e);
                }
            }
        }
    }
    HttpResponse::Ok().json(response_body)
}

fn start_server(port: u16, server_data: Arc<Server>) -> io::Result<()> {
    actix_web::rt::System::new().block_on(async move {
        let server = HttpServer::new(move || {
            App::new()
                .app_data(Data::from(server_data.clone()))
                .service(handle_req)
                .service(handle_pre_prepare)
                .service(handle_prepare)
                .service(handle_commit)
        })
            .bind(("127.0.0.1", port))?;

        println!("Server started on port {}", port);

        server.run().await
    })
}