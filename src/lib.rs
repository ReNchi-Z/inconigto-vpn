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
        Response::from_html("https://inconigto-mode.web.id/")
    }

}

fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    // Extract context data for host and uuid
    let host = cx.data.host.to_string();
    let uuid = cx.data.uuid.to_string();

    // Generate all the required links using helper functions
    let vmess_link = generate_vmess_link(&host, &uuid);
    let vless_link = generate_vless_link(&host, &uuid);
    let trojan_link = generate_trojan_link(&host, &uuid);
    let ss_link = generate_ss_link(&host, &uuid);

    // Create an HTML response string with improved styling
    let html = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Connection Links</title>
            <style>
                * {{
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                }}
                body {{
                    background-color: #f5f5f5;
                    padding: 20px;
                    color: #333;
                }}
                .container {{
                    max-width: 800px;
                    margin: 0 auto;
                    background-color: white;
                    border-radius: 10px;
                    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                    padding: 30px;
                }}
                h1 {{
                    text-align: center;
                    margin-bottom: 30px;
                    color: #2563eb;
                    font-size: 28px;
                }}
                .links-container {{
                    display: grid;
                    gap: 20px;
                }}
                .link-card {{
                    border: 1px solid #e5e7eb;
                    border-radius: 8px;
                    padding: 15px;
                    transition: all 0.3s ease;
                }}
                .link-card:hover {{
                    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
                }}
                .link-header {{
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 10px;
                }}
                .link-title {{
                    font-weight: bold;
                    font-size: 18px;
                    color: #1f2937;
                }}
                .link-content {{
                    position: relative;
                    background-color: #f9fafb;
                    border-radius: 6px;
                    padding: 12px;
                    font-family: monospace;
                    font-size: 14px;
                    word-break: break-all;
                    margin-bottom: 10px;
                    border: 1px solid #e5e7eb;
                }}
                .copy-btn {{
                    background-color: #2563eb;
                    color: white;
                    border: none;
                    border-radius: 6px;
                    padding: 8px 16px;
                    cursor: pointer;
                    font-size: 14px;
                    transition: background-color 0.3s;
                }}
                .copy-btn:hover {{
                    background-color: #1d4ed8;
                }}
                .success-message {{
                    display: none;
                    color: #059669;
                    font-size: 14px;
                    margin-top: 5px;
                }}
                @media (max-width: 600px) {{
                    .container {{
                        padding: 20px;
                    }}
                    .link-content {{
                        font-size: 12px;
                    }}
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Connection Links</h1>
                <div class="links-container">
                    <div class="link-card">
                        <div class="link-header">
                            <span class="link-title">VMess</span>
                            <button class="copy-btn" onclick="copyToClipboard('vmess-link')">Copy</button>
                        </div>
                        <div class="link-content" id="vmess-link">{0}</div>
                        <div class="success-message" id="vmess-success">Copied to clipboard!</div>
                    </div>
                    
                    <div class="link-card">
                        <div class="link-header">
                            <span class="link-title">VLESS</span>
                            <button class="copy-btn" onclick="copyToClipboard('vless-link')">Copy</button>
                        </div>
                        <div class="link-content" id="vless-link">{1}</div>
                        <div class="success-message" id="vless-success">Copied to clipboard!</div>
                    </div>
                    
                    <div class="link-card">
                        <div class="link-header">
                            <span class="link-title">Trojan</span>
                            <button class="copy-btn" onclick="copyToClipboard('trojan-link')">Copy</button>
                        </div>
                        <div class="link-content" id="trojan-link">{2}</div>
                        <div class="success-message" id="trojan-success">Copied to clipboard!</div>
                    </div>
                    
                    <div class="link-card">
                        <div class="link-header">
                            <span class="link-title">Shadowsocks</span>
                            <button class="copy-btn" onclick="copyToClipboard('ss-link')">Copy</button>
                        </div>
                        <div class="link-content" id="ss-link">{3}</div>
                        <div class="success-message" id="ss-success">Copied to clipboard!</div>
                    </div>
                </div>
            </div>

            <script>
                function copyToClipboard(elementId) {{
                    const element = document.getElementById(elementId);
                    const text = element.textContent;
                    
                    navigator.clipboard.writeText(text).then(() => {{
                        // Show success message
                        const successId = elementId.split('-')[0] + '-success';
                        const successElement = document.getElementById(successId);
                        successElement.style.display = 'block';
                        
                        // Hide after 2 seconds
                        setTimeout(() => {{
                            successElement.style.display = 'none';
                        }}, 2000);
                    }}).catch(err => {{
                        console.error('Failed to copy: ', err);
                    }});
                }}
            </script>
        </body>
        </html>
        "#,
        vmess_link, vless_link, trojan_link, ss_link
    );

    // Return HTML response
    Response::from_html(html)
}

/// Generates the vmess link
fn generate_vmess_link(host: &str, uuid: &str) -> String {
    let config = json!({
        "ps": "VMESS",
        "v": "2",
        "add": host,
        "port": "443",
        "id": uuid,
        "aid": "0",
        "scy": "zero",
        "net": "ws",
        "type": "none",
        "host": host,
        "path": "/ID",
        "tls": "true",
        "sni": host,
        "alpn": ""
    });
    format!("vmess://{}", URL_SAFE.encode(config.to_string()))
}

/// Generates the vless link
fn generate_vless_link(host: &str, uuid: &str) -> String {
    format!(
        "vless://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FKR&security=tls&sni={host}#VLESS"
    )
}

/// Generates the trojan link
fn generate_trojan_link(host: &str, uuid: &str) -> String {
    format!(
        "trojan://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FKR&security=tls&sni={host}#TROJAN"
    )
}

/// Generates the ss link
fn generate_ss_link(host: &str, uuid: &str) -> String {
    format!(
        "ss://{}@{host}:443?plugin=v2ray-plugin%3Btls%3Bmux%3D0%3Bmode%3Dwebsocket%3Bpath%3D%2FKR%3Bhost%3D{host}#SS",
        URL_SAFE.encode(format!("none:{uuid}"))
    )
}
