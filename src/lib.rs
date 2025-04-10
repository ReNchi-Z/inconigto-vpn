mod common;
mod config;
mod proxy;

use crate::config::Config;
use crate::proxy::*;

use std::collections::HashMap;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;
use worker::*;
use once_cell::sync::Lazy;
use regex::Regex;

static PROXYIP_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.+-\d+$").unwrap());

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let uuid = env
        .var("UUID")
        .map(|x| Uuid::parse_str(&x.to_string()).unwrap_or_default())?;
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    let main_page_url = env.var("MAIN_PAGE_URL").map(|x|x.to_string()).unwrap();
    let sub_page_url = env.var("SUB_PAGE_URL").map(|x|x.to_string()).unwrap();
    let config = Config { uuid, host: host.clone(), proxy_addr: host, proxy_port: 443, main_page_url, sub_page_url};

    Router::with_data(config)
        .on_async("/", fe)
        .on_async("/sub", sub)
        .on("/link", link)
        .on_async("/:proxyip", tunnel)
        .on_async("/Inconigto-Mode/:proxyip", tunnel)
        .run(req, env)
        .await
}

async fn get_response_from_url(url: String) -> Result<Response> {
    let req = Fetch::Url(Url::parse(url.as_str())?);
    let mut res = req.send().await?;
    Response::from_html(res.text().await?)
}

async fn fe(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.main_page_url).await
}

async fn sub(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.sub_page_url).await
}


async fn tunnel(req: Request, mut cx: RouteContext<Config>) -> Result<Response> {
    let mut proxyip = cx.param("proxyip").unwrap().to_string();
    if proxyip.len() == 2 {
        let req = Fetch::Url(Url::parse("https://raw.githubusercontent.com/FoolVPN-ID/Nautica/refs/heads/main/kvProxyList.json")?);
        let mut res = req.send().await?;
        if res.status_code() == 200 {
            let proxy_kv: HashMap<String, Vec<String>> = serde_json::from_str(&res.text().await?)?;
            proxyip = proxy_kv[&proxyip][0].clone().replace(":", "-");
        }
    }

    if PROXYIP_PATTERN.is_match(&proxyip) {
        if let Some((addr, port_str)) = proxyip.split_once('-') {
            if let Ok(port) = port_str.parse() {
                cx.data.proxy_addr = addr.to_string();
                cx.data.proxy_port = port;
            }
        }
    }
    
    let upgrade = req.headers().get("Upgrade")?.unwrap_or("".to_string());
    if upgrade == "websocket".to_string() {
        let WebSocketPair { server, client } = WebSocketPair::new()?;
        server.accept()?;
    
        wasm_bindgen_futures::spawn_local(async move {
            let events = server.events().unwrap();
            if let Err(e) = ProxyStream::new(cx.data, &server, events).process().await {
                console_log!("[tunnel]: {}", e);
            }
        });
    
        Response::from_websocket(client)
    } else {
        Response::from_html("hi from wasm!")
    }

}

fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    let host = cx.data.host.to_string();
    let uuid = cx.data.uuid.to_string();

    let vmess_link = generate_vmess_link(&host, &uuid);
    let vless_link = generate_vless_link(&host, &uuid);
    let trojan_link = generate_trojan_link(&host, &uuid);
    let ss_link = generate_ss_link(&host, &uuid);

    let template = include_str!("templates/template.html");

    let html = template
        .replace("{{vmess}}", &vmess_link)
        .replace("{{vless}}", &vless_link)
        .replace("{{trojan}}", &trojan_link)
        .replace("{{ss}}", &ss_link);

    Response::from_html(html)
}


/// Generates the vmess link
fn generate_vmess_link(host: &str, uuid: &str) -> String {
    let config = json!({
        "ps": "siren vmess",
        "v": "2",
        "add": host,
        "port": "80",
        "id": uuid,
        "aid": "0",
        "scy": "zero",
        "net": "ws",
        "type": "none",
        "host": host,
        "path": "/KR",
        "tls": "",
        "sni": "",
        "alpn": ""
    });
    format!("vmess://{}", URL_SAFE.encode(config.to_string()))
}

/// Generates the vless link
fn generate_vless_link(host: &str, uuid: &str) -> String {
    format!(
        "vless://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FKR&security=tls&sni={host}#siren vless"
    )
}

/// Generates the trojan link
fn generate_trojan_link(host: &str, uuid: &str) -> String {
    format!(
        "trojan://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FKR&security=tls&sni={host}#siren trojan"
    )
}

/// Generates the ss link
fn generate_ss_link(host: &str, uuid: &str) -> String {
    format!(
        "ss://{}@{host}:443?plugin=v2ray-plugin%3Btls%3Bmux%3D0%3Bmode%3Dwebsocket%3Bpath%3D%2FKR%3Bhost%3D{host}#siren ss",
        URL_SAFE.encode(format!("none:{uuid}"))
    )
}
