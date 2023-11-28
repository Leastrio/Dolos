use std::convert::Infallible;

use http_body_util::Full;
use hyper::{server::conn::http1, service::service_fn, Request, body::{Incoming, Bytes}, Response};
use hyper_util::rt::TokioIo;
use jsonwebtoken::{Validation, Algorithm, DecodingKey, decode};
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::OnceCell};

use crate::{DolosResult, tcp_proxy::{CHAT_PORT, INIT_NOTIFY}};

const CONFIG_URL: &str = "https://clientconfig.rpg.riotgames.com";
const GEO_PAS_URL: &str = "https://riot-geo.pas.si.riotgames.com/pas/v1/service/chat";

pub static RIOT_CHAT: OnceCell<RiotChat> = OnceCell::const_new();
pub static HTTP_PORT: OnceCell<u64> = OnceCell::const_new();

#[derive(Debug)]
pub struct RiotChat {
    pub host: String,
    pub port: u64
}


pub async fn listen_http() -> DolosResult<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    HTTP_PORT.set(listener.local_addr()?.port().into())?;
    println!("[Dolos] [HTTP] Listening on {}", listener.local_addr()?);
    
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.expect("[Dolos] [HTTP] Could not accept connection");
            let io = TokioIo::new(stream);
            
            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(process))
                    .await
                {
                    println!("[Dolos] [HTTP] Error serving connection: {:?}", err);
                }
            });
        }
    });
    Ok(())
}

async fn process(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let client = reqwest::Client::new();
    let url = CONFIG_URL.to_owned() + &req.uri().to_string();
    println!("[Dolos] [HTTP] Sending Request to {}", url);
    let mut headers = HeaderMap::new();

    if let Some(user_agent) = req.headers().get("user-agent") {
        headers.append("user-agent", user_agent.to_str().expect("[Dolos] [HTTP] Header value is not a valid string").to_string().parse().expect("[Dolos] [HTTP] Could not parse header value"));
    }
    if let Some(jwt) = req.headers().get("x-riot-entitlements-jwt") {
        headers.append("x-riot-entitlements-jwt", jwt.to_str().expect("[Dolos] [HTTP] Header value is not a valid string").to_string().parse().expect("[Dolos] [HTTP] Could not parse header value"));
    }
    if let Some(auth) = req.headers().get("authorization") {
        headers.append("authorization", auth.to_str().expect("[Dolos] [HTTP] Header value is not a valid string").to_string().parse().expect("[Dolos] [HTTP] Could not parse header value"));
    }

    let resp = client.get(url)
        .headers(headers.clone())
        .send()
        .await
        .expect("[Dolos] [HTTP] Could not make request");

    let reply = if resp.status() == 200 {
        let data: Value = serde_json::from_slice(&resp.bytes().await.expect("[Dolos] [HTTP] Could not get bytes")).expect("[Dolos] [HTTP] Invalid JSON");
        rewrite_resp(data, headers).await
    } else {
        resp.bytes().await.expect("[Dolos] [HTTP] Could not get bytes")
    };
    
    Ok(Response::new(Full::new(reply)))
}

async fn rewrite_resp(mut resp: Value, headers: HeaderMap) -> Bytes {
    let mut riot_chat_port: u64 = 0;
    let mut riot_chat_host = String::new();

    if let Some(host) = resp.get("chat.host") {
        riot_chat_host = host.to_string();
        resp["chat.host"] = json!("127.0.0.1");
    }

    if let Some(port) = resp.get("chat.port") {
        riot_chat_port = port.as_u64().unwrap();
        resp["chat.port"] = json!(CHAT_PORT.get().unwrap());
    }

    if let Some(_) = resp.get("chat.affinities") {
        if let Some(_) = resp.get("chat.affinity.enabled") {
            if !RIOT_CHAT.initialized() {
                let pas = reqwest::Client::new()
                    .get(GEO_PAS_URL)
                    .headers(headers)
                    .send()
                    .await
                    .expect("[Dolos] [HTTP] Could not fetch pas token")
                    .text()
                    .await
                    .expect("[Dolos] [HTTP] could not fetch body from pas req");

                let mut validation = Validation::new(Algorithm::HS256);
                validation.insecure_disable_signature_validation();

                let jwt = decode::<Value>(&pas, &DecodingKey::from_secret(&[]), &validation).expect("[Dolos] [HTTP] Could not decode jwt");
                riot_chat_host = resp["chat.affinities"][jwt.claims["affinity"].as_str().unwrap()].to_string();
            }
        }
        for (region, _) in resp["chat.affinities"].clone().as_object().unwrap().iter() {
            resp["chat.affinities"][region] = json!("127.0.0.1")
        };
    }

    if let Some(_) = resp.get("chat.allow_bad_cert.enabled") {
        resp["chat.allow_bad_cert.enabled"] = json!(true);
    }

    if riot_chat_port != 0 && !riot_chat_host.is_empty() && !RIOT_CHAT.initialized() {
        RIOT_CHAT.set(RiotChat { host: riot_chat_host.trim_start_matches("\"").trim_end_matches("\"").to_string(), port: riot_chat_port }).expect("[Dolos] [HTTP] RIOT_CHAT OnceCell written to twice");
        INIT_NOTIFY.notify_one();
    }
    Bytes::from(serde_json::to_vec(&resp).expect("[Dolos] [HTTP] Could not serialize to vec"))
}