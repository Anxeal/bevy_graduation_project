use std::net::TcpStream;

use bevy::{prelude::*, utils::Instant};
use bincode::{deserialize, serialize};
use shared::*;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};
use url::Url;

use human_bytes::human_bytes;

use crate::error::Result;

#[derive(Resource)]
pub struct PhysicsClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl PhysicsClient {
    pub fn new(url: Url) -> Self {
        println!("Connecting to {}", url);
        let (socket, response) = connect(url).expect("Can't connect to physics server");

        println!("Connected to the server");
        println!("Response HTTP code: {}", response.status());
        println!("Response contains the following headers:");
        for (ref header, _value) in response.headers() {
            println!("* {}", header);
        }

        Self { socket }
    }

    pub fn send_request(&mut self, request: Request) -> Result<Response> {
        let msg = Message::Binary(serialize(&request)?);
        let msg_len = msg.len();

        trace!(
            "Sending request ({}): {:?}",
            human_bytes(msg_len as f64),
            request
        );

        let start = Instant::now();
        self.socket.write_message(msg.clone())?;

        let msg = self.socket.read_message()?;
        let msg_len = msg.len();

        let response = deserialize::<Response>(&msg.into_data())?;

        trace!(
            "Received response ({}) in {:?}: {:?}",
            human_bytes(msg_len as f64),
            start.elapsed(),
            response
        );

        Ok(response)
    }
}
