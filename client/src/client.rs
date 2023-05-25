use std::{
    io::{Read, Write},
    net::TcpStream,
};

use bevy::{prelude::*, utils::Instant};
use bincode::{deserialize, serialize};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use shared::*;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};
use url::Url;

use human_bytes::human_bytes;

use crate::error::Result;

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
        let serialized = serialize(&request)?;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialized)?;
        let compressed = encoder.finish()?;

        let msg = Message::Binary(compressed);
        let msg_len = msg.len();

        debug!("Sending request ({})", human_bytes(msg_len as f64));
        trace!("Sending request: {:?}", request);

        let start = Instant::now();
        self.socket.write_message(msg.clone())?;

        let msg = self.socket.read_message()?;
        let msg_len = msg.len();
        let msg_data = msg.into_data();

        let mut decoder = ZlibDecoder::new(msg_data.as_slice());
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        let response = deserialize::<Response>(decompressed.as_slice())?;

        debug!(
            "Received response ({}) in {:?}",
            human_bytes(msg_len as f64),
            start.elapsed()
        );
        trace!("Received response: {:?}", response);

        Ok(response)
    }
}
