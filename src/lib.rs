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

    // Create an HTML response with a tech-themed design
    let html = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Connection Hub</title>
            <link rel="preconnect" href="https://fonts.googleapis.com">
            <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
            <link href="https://fonts.googleapis.com/css2?family=Rajdhani:wght@500;600;700&family=Roboto+Mono&display=swap" rel="stylesheet">
            <style>
                :root {{
                    --bg-color: #0a0e17;
                    --card-bg: #141c2e;
                    --primary: #00ccff;
                    --primary-glow: rgba(0, 204, 255, 0.5);
                    --secondary: #ff00aa;
                    --secondary-glow: rgba(255, 0, 170, 0.5);
                    --text: #e6f1ff;
                    --text-muted: #8a9cc2;
                    --border: #1e2a45;
                    --success: #00ff9d;
                }}
                
                * {{
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                }}
                
                body {{
                    background-color: var(--bg-color);
                    background-image: 
                        radial-gradient(circle at 20% 20%, rgba(0, 204, 255, 0.03) 0%, transparent 40%),
                        radial-gradient(circle at 80% 80%, rgba(255, 0, 170, 0.03) 0%, transparent 40%),
                        linear-gradient(rgba(2, 13, 30, 0.7) 1px, transparent 1px),
                        linear-gradient(90deg, rgba(2, 13, 30, 0.7) 1px, transparent 1px);
                    background-size: 100% 100%, 100% 100%, 20px 20px, 20px 20px;
                    background-position: 0 0, 0 0, -1px -1px, -1px -1px;
                    color: var(--text);
                    min-height: 100vh;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    padding: 20px;
                    font-family: 'Rajdhani', sans-serif;
                }}
                
                .container {{
                    max-width: 800px;
                    width: 100%;
                    background-color: var(--card-bg);
                    border-radius: 12px;
                    box-shadow: 
                        0 0 30px rgba(0, 0, 0, 0.5),
                        0 0 15px var(--primary-glow),
                        0 0 25px var(--secondary-glow);
                    overflow: hidden;
                    position: relative;
                    z-index: 1;
                }}
                
                .container::before {{
                    content: '';
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    height: 1px;
                    background: linear-gradient(90deg, transparent, var(--primary), transparent);
                    z-index: 2;
                }}
                
                .container::after {{
                    content: '';
                    position: absolute;
                    bottom: 0;
                    left: 0;
                    right: 0;
                    height: 1px;
                    background: linear-gradient(90deg, transparent, var(--secondary), transparent);
                    z-index: 2;
                }}
                
                .header {{
                    background: linear-gradient(135deg, #141c2e 0%, #1a2540 100%);
                    padding: 25px 30px;
                    text-align: center;
                    position: relative;
                    overflow: hidden;
                }}
                
                .header::before {{
                    content: '';
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    background: 
                        linear-gradient(90deg, transparent, rgba(0, 204, 255, 0.1), transparent) no-repeat;
                    background-size: 50% 100%;
                    background-position: -100% 0;
                    animation: header-shine 5s infinite;
                }}
                
                @keyframes header-shine {{
                    0% {{ background-position: -100% 0; }}
                    40%, 100% {{ background-position: 200% 0; }}
                }}
                
                h1 {{
                    font-weight: 700;
                    font-size: 28px;
                    color: white;
                    margin: 0;
                    letter-spacing: 1px;
                    text-transform: uppercase;
                    position: relative;
                    display: inline-block;
                }}
                
                h1::after {{
                    content: '';
                    position: absolute;
                    bottom: -8px;
                    left: 50%;
                    transform: translateX(-50%);
                    width: 40px;
                    height: 3px;
                    background: linear-gradient(90deg, var(--primary), var(--secondary));
                }}
                
                .content {{
                    padding: 30px;
                }}
                
                .links-grid {{
                    display: grid;
                    gap: 20px;
                }}
                
                .link-card {{
                    background-color: rgba(20, 28, 46, 0.7);
                    border: 1px solid var(--border);
                    border-radius: 8px;
                    padding: 20px;
                    transition: all 0.3s ease;
                    position: relative;
                    overflow: hidden;
                }}
                
                .link-card::before {{
                    content: '';
                    position: absolute;
                    top: 0;
                    left: 0;
                    width: 4px;
                    height: 100%;
                    background: linear-gradient(to bottom, var(--primary), var(--secondary));
                }}
                
                .link-card:hover {{
                    transform: translateY(-3px);
                    box-shadow: 
                        0 10px 20px rgba(0, 0, 0, 0.2),
                        0 0 10px var(--primary-glow);
                    border-color: var(--primary);
                }}
                
                .link-header {{
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 15px;
                }}
                
                .link-title {{
                    font-weight: 600;
                    font-size: 18px;
                    color: var(--primary);
                    display: flex;
                    align-items: center;
                    letter-spacing: 0.5px;
                }}
                
                .link-title::before {{
                    content: '⬢';
                    margin-right: 8px;
                    font-size: 14px;
                    color: var(--primary);
                }}
                
                .link-content {{
                    background-color: rgba(10, 14, 23, 0.8);
                    border: 1px solid var(--border);
                    border-radius: 6px;
                    padding: 12px;
                    font-family: 'Roboto Mono', monospace;
                    font-size: 13px;
                    color: var(--text-muted);
                    word-break: break-all;
                    margin-bottom: 10px;
                    position: relative;
                    overflow: hidden;
                }}
                
                .link-content::after {{
                    content: '';
                    position: absolute;
                    top: 0;
                    right: 0;
                    width: 30px;
                    height: 100%;
                    background: linear-gradient(to left, rgba(20, 28, 46, 1), transparent);
                    pointer-events: none;
                }}
                
                .copy-btn {{
                    background: linear-gradient(135deg, var(--primary), var(--secondary));
                    color: white;
                    border: none;
                    border-radius: 6px;
                    padding: 8px 16px;
                    cursor: pointer;
                    font-size: 14px;
                    font-weight: 600;
                    font-family: 'Rajdhani', sans-serif;
                    letter-spacing: 0.5px;
                    transition: all 0.2s ease;
                    position: relative;
                    overflow: hidden;
                }}
                
                .copy-btn::before {{
                    content: '';
                    position: absolute;
                    top: 0;
                    left: -100%;
                    width: 100%;
                    height: 100%;
                    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.2), transparent);
                    transition: all 0.6s ease;
                }}
                
                .copy-btn:hover::before {{
                    left: 100%;
                }}
                
                .copy-btn:hover {{
                    box-shadow: 0 0 15px var(--primary-glow);
                    transform: translateY(-2px);
                }}
                
                .copy-btn:active {{
                    transform: translateY(0);
                }}
                
                .success-message {{
                    display: none;
                    color: var(--success);
                    font-size: 14px;
                    margin-top: 8px;
                    text-align: right;
                    font-weight: 600;
                }}
                
                @media (max-width: 600px) {{
                    .header {{
                        padding: 20px;
                    }}
                    
                    h1 {{
                        font-size: 24px;
                    }}
                    
                    .content {{
                        padding: 20px;
                    }}
                    
                    .link-card {{
                        padding: 15px;
                    }}
                    
                    .link-content {{
                        font-size: 12px;
                        padding: 10px;
                    }}
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>Connection Hub</h1>
                </div>
                <div class="content">
                    <div class="links-grid">
                        <div class="link-card">
                            <div class="link-header">
                                <span class="link-title">VMess</span>
                                <button class="copy-btn" onclick="copyToClipboard('vmess-link')">Copy</button>
                            </div>
                            <div class="link-content" id="vmess-link">{0}</div>
                            <div class="success-message" id="vmess-success">✓ Connection data copied</div>
                        </div>
                        
                        <div class="link-card">
                            <div class="link-header">
                                <span class="link-title">VLESS</span>
                                <button class="copy-btn" onclick="copyToClipboard('vless-link')">Copy</button>
                            </div>
                            <div class="link-content" id="vless-link">{1}</div>
                            <div class="success-message" id="vless-success">✓ Connection data copied</div>
                        </div>
                        
                        <div class="link-card">
                            <div class="link-header">
                                <span class="link-title">Trojan</span>
                                <button class="copy-btn" onclick="copyToClipboard('trojan-link')">Copy</button>
                            </div>
                            <div class="link-content" id="trojan-link">{2}</div>
                            <div class="success-message" id="trojan-success">✓ Connection data copied</div>
                        </div>
                        
                        <div class="link-card">
                            <div class="link-header">
                                <span class="link-title">Shadowsocks</span>
                                <button class="copy-btn" onclick="copyToClipboard('ss-link')">Copy</button>
                            </div>
                            <div class="link-content" id="ss-link">{3}</div>
                            <div class="success-message" id="ss-success">✓ Connection data copied</div>
                        </div>
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
        "vless://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FID&security=tls&sni={host}#VLESS"
    )
}

/// Generates the trojan link
fn generate_trojan_link(host: &str, uuid: &str) -> String {
    format!(
        "trojan://{uuid}@{host}:443?encryption=none&type=ws&host={host}&path=%2FID&security=tls&sni={host}#TROJAN"
    )
}

/// Generates the ss link
fn generate_ss_link(host: &str, uuid: &str) -> String {
    format!(
        "ss://{}@{host}:443?plugin=v2ray-plugin%3Btls%3Bmux%3D0%3Bmode%3Dwebsocket%3Bpath%3D%2FID%3Bhost%3D{host}#SS",
        URL_SAFE.encode(format!("none:{uuid}"))
    )
}
