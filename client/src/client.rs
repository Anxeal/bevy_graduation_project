use std::{
    io::{Read, Write},
    net::TcpStream,
};

use bevy::{prelude::*, utils::Instant};
use bincode::{deserialize, serialize};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression, Decompress};
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

        let msg = {
            #[cfg(feature = "compression")]
            {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&serialized)?;
                let compressed = encoder.finish()?;

                Message::Binary(compressed)
            }
            #[cfg(not(feature = "compression"))]
            {
                Message::Binary(serialized)
            }
        };

        let msg_len = msg.len();
        let request_type = request.name();

        debug!(
            msg_len,
            request_type,
            "Sending request <{}> ({})",
            request_type,
            human_bytes(msg_len as f64)
        );
        trace!("Sending request: {:?}", request);

        let start = Instant::now();
        self.socket.write_message(msg)?;

        let msg = self.socket.read_message()?;
        let msg_len = msg.len();
        let msg_data = msg.into_data();

        let serialized = {
            #[cfg(feature = "compression")]
            {
                let mut decoder = ZlibDecoder::new(msg_data.as_slice());
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;

                decompressed
            }
            #[cfg(not(feature = "compression"))]
            {
                msg_data
            }
        };
        let response = deserialize::<Response>(serialized.as_slice())?;
        let response_type = response.name();
        let elapsed = start.elapsed();

        debug!(
            msg_len,
            response_type,
            latency_in_nanos = elapsed.as_nanos(),
            "Received response <{}> ({}) in {:?}",
            response_type,
            human_bytes(msg_len as f64),
            elapsed
        );
        trace!("Received response: {:?}", response);

        Ok(response)
    }
}
