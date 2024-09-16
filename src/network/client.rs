use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::thread::JoinHandle;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use actix_web::web::Data;
use serde_json::json;
use crate::consensus::message::RequestMsg;
use futures::future::join_all;

#[derive(Clone)]
pub(crate) struct Client {
    server_table: HashMap<u32, String>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Client {
    pub(crate) fn new(n: u32) -> Self {
        let mut server_table = HashMap::new();
        let mut base_port = 8000;
        for i in 0..n {
            server_table.insert(i, "127.0.0.1:".to_string() + &(base_port + i).to_string());
        }
        Self {
            server_table,
            handle: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn start(&mut self) {
        let client_data = Arc::new(self.clone());
        let client_data_clone = client_data.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = start_client(client_data_clone) {
                eprintln!("Client failed to start on port {}: {}", 9000, e);
            }
        });
        let mut handle_lock = self.handle.lock().unwrap();
        *handle_lock = Some(handle);
    }


    fn join(self) {
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
async fn client_handle_req(request_msg: web::Json<RequestMsg>, client_data: Data<Client>) -> impl Responder {
    println!("[💻 Client] Received RequestMsg: {:?}", request_msg);
    let server_table = &client_data.server_table;
    let client = reqwest::Client::new();

    // Collect all the requests into a vector of futures
    let requests: Vec<_> = server_table.iter().map(|(id, server_address)| {
        let request_msg_clone = request_msg.clone();
        let client_clone = client.clone();
        let server_address_clone = server_address.clone();
        let id_clone = *id;

        // Return a future representing the request
        async move {
            println!("[💻 Client] Sending request to server {}: {}", id_clone, server_address_clone);
            match client_clone.post(&format!("http://{}/req", server_address_clone))
                .json(&request_msg_clone)
                .send()
                .await
            {
                Ok(response) => {
                    // Handle the response here if needed
                    // println!("  -- Response from server {}: {:?}", id_clone, response.status());
                }
                Err(e) => {
                    // eprintln!(" -- Error sending request to node {}: {}", id_clone, e);
                }
            }
        }
    }).collect();

    // Run all requests concurrently
    join_all(requests).await;

    HttpResponse::Ok().json(json!({"status": "client ok"}))
}

fn start_client(client_data: Arc<Client>) -> io::Result<()> {
    actix_web::rt::System::new().block_on(async move {
        let client_server = HttpServer::new(move || {
            App::new()
                .app_data(Data::from(client_data.clone()))
                .service(client_handle_req)
        })
            .bind(("127.0.0.1", 9000))?; // client server runs on port 9000

        println!("Client started on port {}", 9000);

        client_server.run().await
    })
}