use std::collections::HashMap;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde_json::json;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crate::network::server::Server;
use actix_web::web::Data;
use crate::consensus::message::RequestMsg;
use crate::network::node::Node;
use crate::network::client::Client;


// ============================== Client ==============================




// ============================== Server ==============================





// ============================== Launch Client & Servers==============================
pub fn launch(n: u32) -> io::Result<()> {
    let mut servers = Vec::new();

    for i in 0..n {
        let port = 8000 + i;
        let mut server = Server::new(i, port as u16, n);
        server.start();
        servers.push(server);
    }
    let mut client = Client::new(n);
    client.start();
    // wait for all server threads (servers will run indefinitely)
    for server in servers {
        server.join();
    }

    Ok(())
}