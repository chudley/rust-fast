/*
 * Copyright 2018 Joyent, Inc.
 */


extern crate chrono;
extern crate rust_fast;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio;

use std::env;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::prelude::*;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::prelude::*;

use rust_fast::protocol::{FastMessage, FastMessageData};
use rust_fast::server;

#[derive(Serialize, Deserialize)]
struct YesPayload {
    value: Value,
    count: u32
}

#[derive(Serialize, Deserialize)]
struct DatePayload {
    timestamp: u64,
    iso8601: DateTime<Utc>
}

impl DatePayload {
    fn new() -> DatePayload {
        //TODO: Do this only with chrono and time libs
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let now_micros = now.as_secs() * 1_000
            + now.subsec_millis() as u64;
        let now2 = Utc::now();
        DatePayload {
            timestamp: now_micros,
            iso8601: now2
        }
    }
}

fn other_error(msg: &str) -> Error {
    Error::new(ErrorKind::Other, String::from(msg))
}

fn date_handler(msg: &FastMessage, mut response: Vec<FastMessage>) -> Result<Vec<FastMessage>, Error> {
    let date_payload_result = serde_json::to_value(vec![DatePayload::new()]);
    match date_payload_result {
        Ok(date_payload) => {
            response.push(FastMessage::data(msg.id, FastMessageData::new(msg.data.m.name.clone(), date_payload)));
            Ok(response)
        },
        Err(_) =>
            Err(other_error("Failed to parse JSON data as payload for date function"))
    }
}

fn echo_handler(msg: &FastMessage, mut response: Vec<FastMessage>) -> Result<Vec<FastMessage>, Error> {
    response.push(FastMessage::data(msg.id, msg.data.clone()));
    Ok(response)
}


fn yes_handler(msg: &FastMessage, mut response: Vec<FastMessage>) -> Result<Vec<FastMessage>, Error> {
    //TODO: Too much nesting, need to refactor
    match msg.data.d {
        Value::Array(_) => {
            let data_clone = msg.data.clone();
            let payload_result: Result<Vec<YesPayload>, _> = serde_json::from_value(data_clone.d);
            match payload_result {
                Ok(payloads) => {
                    if payloads.len() == 1 {
                        for _i in 0..payloads[0].count {
                            let value = Value::Array(vec![payloads[0].value.clone()]);
                            let yes_data = FastMessage::data(msg.id, FastMessageData::new(msg.data.m.name.clone(), value));
                            response.push(yes_data);
                        }
                        Ok(response)
                    } else {
                        Err(other_error("Expected JSON array with a single element"))
                    }
                },
                Err(_) =>
                    Err(other_error("Failed to parse JSON data as payload for yes function"))
            }
        }
        _ => Err(other_error("Expected JSON array"))
    }
}

fn msg_handler(msg: &FastMessage) -> Result<Vec<FastMessage>, Error> {
    let response: Vec<FastMessage> = vec![];

    match msg.data.m.name.as_str() {
        "date" => date_handler(msg, response),
        "echo" => echo_handler(msg, response),
        "yes" => yes_handler(msg, response),
        _ => Err(Error::new(ErrorKind::Other, format!("Unsupport functon: {}", msg.data.m.name)))
    }
}

fn main() {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:2030".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();

    let listener = TcpListener::bind(&addr).expect("failed to bind");
    println!("Listening on: {}", addr);

    tokio::run({
        listener.incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(|socket| {
                server::process(socket, Arc::new(msg_handler));
                Ok(())
            })
    });
}
