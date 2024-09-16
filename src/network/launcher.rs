use std::io;
use crate::network::server::Server;
use crate::network::client::Client;

pub fn launch(n: u32, f: u32) -> io::Result<()> {
    let mut servers = Vec::new();
    for i in 0..n {
        let mut is_faulty = false;
        if i >= n - f {
            is_faulty = true;
        }
        let port = 8000 + i;
        let mut server = Server::new(i, port as u16, n, is_faulty);
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