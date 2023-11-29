use std::{sync::Arc, io::Cursor};

use native_tls::Identity;
use quick_xml::{Reader, Writer, events::{Event, BytesText}};
use tokio::{net::{TcpStream, TcpListener}, sync::{mpsc, OnceCell, Notify}, io::{AsyncReadExt, AsyncWriteExt}};
use tokio_native_tls::TlsStream;

use crate::{DolosResult, http_proxy::RIOT_CHAT};

pub static CHAT_PORT: OnceCell<u64> = OnceCell::const_new();
pub static INIT_NOTIFY: Notify = Notify::const_new();

struct InputActor {
    input_stream: TlsStream<TcpStream>,
    output_sender: mpsc::Sender<Vec<u8>>,
    input_reciever: mpsc::Receiver<Vec<u8>>
}

struct OutputActor {
    output_stream: TlsStream<TcpStream>,
    input_sender: mpsc::Sender<Vec<u8>>,
    output_reciever: mpsc::Receiver<Vec<u8>>
}

// Server for Riot Client
impl InputActor {
    async fn run(mut self) {
        loop {
            let mut curr_buf = [0; 1024 * 64];
            tokio::select! {
                // Receiving from Output Actor
                Some(data) = self.input_reciever.recv() => {
                    let _ = self.input_stream.write_all(&data).await;
                }
                // Receiving from Riot Client
                Ok(n) = self.input_stream.read(&mut curr_buf) => {
                    if n > 0 {
                        let _ = self.output_sender.send(curr_buf[0..n].to_vec()).await;
                    }
                }
            }
        }
    }
}

// Connection to Riot Servers
impl OutputActor {
    async fn run(mut self) {
        loop {
            let mut curr_buf = [0; 1024 * 64]; 
            tokio::select! {
                // Receiving from Input Actor
                Some(data) = self.output_reciever.recv() => {
                    let data_str = String::from_utf8_lossy(&data);
                    let msg = if data_str.contains("<presence") {
                        rewrite_presence(&data_str).await
                    } else {
                        data
                    };
                    let _ = self.output_stream.write_all(&msg).await;
                }
                // Receiving from Riot Servers
                Ok(n) = self.output_stream.read(&mut curr_buf) => {
                    if n > 0 {
                        let _ = self.input_sender.send(curr_buf[0..n].to_vec()).await;
                    }
                }
            }
        }
    }
}

pub async fn proxy_tcp_chat() -> DolosResult<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    CHAT_PORT.set(listener.local_addr()?.port().into())?;
    println!("[Dolos] [TCP] Listening on {}", listener.local_addr()?);

    let cert = include_bytes!("../../certs/server.cert");
    let key = include_bytes!("../../certs/server.key");
    let cert = Identity::from_pkcs8(cert, key)?;

    let acceptor = Arc::new(tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::new(cert)?));


    INIT_NOTIFY.notified().await;
    println!("[Dolos] [TCP] Continuing...");

    loop {
        let (stream, _) = listener.accept().await.expect("[Dolos] [TCP] Could not accept connection");
        let input_stream = (acceptor.clone()).accept(stream).await.expect("[Dolos] [TCP] Could not accept connection via tls");

        let riot_chat = RIOT_CHAT.get().unwrap();
        let output_stream = TcpStream::connect(format!("{}:{}", riot_chat.host, riot_chat.port)).await.expect("[Dolos] [TCP] Could not connect to riot chat server");
        let connector = tokio_native_tls::TlsConnector::from(native_tls::TlsConnector::new().expect("[Dolos] [TCP] Could not create tls connector"));
        let output_stream = connector.connect(&riot_chat.host, output_stream).await.expect("[Dolos] [TCP] Could not connect tls to riot chat server");

        let (input_sender, input_reciever) = mpsc::channel::<Vec<u8>>(100);
        let (output_sender, output_reciever) = mpsc::channel::<Vec<u8>>(100);

        let input_actor = InputActor {
            input_stream,
            output_sender,
            input_reciever
        };
        let output_actor = OutputActor {
            output_stream,
            input_sender,
            output_reciever
        };

        tokio::spawn(input_actor.run());
        tokio::spawn(output_actor.run());
    }
}

async fn rewrite_presence(data: &str) -> Vec<u8> {
    println!("[Dolos] [TCP] Rewriting Presence Update");
    let mut reader = Reader::from_str(data);
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut inside_show = false;
    let mut inside_game = false;
    let mut inside_game_st = false;

    loop {
        match reader.read_event() {
            // Tag Starts
            Ok(Event::Start(e)) if e.name().as_ref() == b"show" => {
                inside_show = true;
                writer.write_event_async(Event::Start(e.clone())).await.unwrap();
            },
            Ok(Event::Start(e)) if e.name().as_ref() == b"league_of_legends" || e.name().as_ref() == b"valorant" => {
                inside_game = true;
                writer.write_event_async(Event::Start(e.clone())).await.unwrap();
            },
            Ok(Event::Start(e)) if inside_game && e.name().as_ref() == b"st" => {
                inside_game_st = true;
                writer.write_event_async(Event::Start(e.clone())).await.unwrap();
            },
            Ok(Event::Start(e)) if e.name().as_ref() == b"status" => {},

            // Tag insides
            Ok(Event::Text(_)) if inside_show => {
                writer.write_event_async(Event::Text(BytesText::new("offline"))).await.unwrap();
            },
            Ok(Event::Text(_)) if inside_game && inside_game_st => {
                writer.write_event_async(Event::Text(BytesText::new("offline"))).await.unwrap();
            },

            // Tag ends
            Ok(Event::End(e)) if inside_show => {
                inside_show = false;
                writer.write_event_async(Event::End(e)).await.unwrap();
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"league_of_legends" || e.name().as_ref() == b"valorant" => {
                inside_game = false;
                writer.write_event_async(Event::End(e)).await.unwrap();
            },
            Ok(Event::End(e)) if inside_game_st && e.name().as_ref() == b"st" => {
                inside_game_st = false;
                writer.write_event_async(Event::End(e)).await.unwrap();
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"status" => {},

            // Other
            Ok(Event::Eof) => break,
            Ok(e) => writer.write_event_async(e).await.unwrap(),
            Err(e) => {
                eprintln!("[Dolos] [TCP] Error while parsing xml: {}", e)
            }
        }
    }
    writer.into_inner().into_inner()
}