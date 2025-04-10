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
    // Create an HTML response string with basic structure
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Inconigto-Mode</title>
<link rel="icon" href="https://raw.githubusercontent.com/AFRcloud/BG/main/icons8-film-noir-80.png">
<meta property="og:image:secure_url" content="https://raw.githubusercontent.com/akulelaki696/bg/refs/heads/main/20250106_010158.jpg"/>
<meta property="og:audio" content="URL-to-audio-if-any"/>
<meta property="og:video" content="URL-to-video-if-any"/>
<meta name="theme-color" content="#0f172a" media="(prefers-color-scheme: dark)"/>
<meta name="theme-color" content="#f8f9fa" media="(prefers-color-scheme: light)"/>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
<!-- QR Code Library -->
<script src="https://cdn.jsdelivr.net/npm/qrcode@1.5.3/build/qrcode.min.js"></script>
<!-- Add Inter font -->
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
<style>
/* Update the root variables with more vibrant colors */
:root {
    --primary: #6366f1;
    --primary-dark: #4f46e5;
    --primary-light: #818cf8;
    --secondary: #10b981;
    --secondary-dark: #059669;
    --accent: #f43f5e;
    --dark: #0f172a;
    --dark-light: #1e293b;
    --light: #f8fafc;
    --gray: #64748b;
    --gray-light: #94a3b8;
    --gray-dark: #334155;
}

body {
    background-color: var(--dark);
    background-image: 
        radial-gradient(circle at 10% 10%, rgba(99, 102, 241, 0.15) 0%, transparent 50%),
        radial-gradient(circle at 90% 90%, rgba(16, 185, 129, 0.15) 0%, transparent 50%),
        radial-gradient(circle at 90% 10%, rgba(244, 63, 94, 0.1) 0%, transparent 50%),
        radial-gradient(circle at 10% 90%, rgba(56, 189, 248, 0.1) 0%, transparent 50%);
    color: var(--light);
    min-height: 100vh;
}

.glass-card {
    background: rgba(30, 41, 59, 0.7);
    backdrop-filter: blur(16px);
    border: 1px solid rgba(99, 102, 241, 0.3);
    box-shadow: 
        0 10px 25px -3px rgba(0, 0, 0, 0.2),
        0 4px 6px -2px rgba(0, 0, 0, 0.1),
        0 0 0 1px rgba(99, 102, 241, 0.2),
        inset 0 1px 1px rgba(255, 255, 255, 0.05);
    border-radius: 20px;
    transition: all 0.3s ease;
}

.glass-card:hover {
    box-shadow: 
        0 15px 30px -5px rgba(0, 0, 0, 0.3),
        0 4px 6px -2px rgba(0, 0, 0, 0.1),
        0 0 0 1px rgba(99, 102, 241, 0.3),
        inset 0 1px 1px rgba(255, 255, 255, 0.1);
    transform: translateY(-2px);
}

.glass-input {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(99, 102, 241, 0.3);
    color: var(--light);
    transition: all 0.3s ease;
    border-radius: 10px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
}

.glass-input:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.3), inset 0 2px 4px rgba(0, 0, 0, 0.1);
    background: rgba(15, 23, 42, 0.9);
}

.glass-input[readonly] {
    background: rgba(51, 65, 85, 0.4);
}

.glass-select {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(99, 102, 241, 0.2);
    color: var(--light);
    border-radius: 8px;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='%2394a3b8'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 9l-7 7-7-7'%3E%3C/path%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    background-size: 1rem;
    padding-right: 2.5rem;
}

.glass-select:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.25);
    background-color: rgba(15, 23, 42, 0.8);
}

.primary-btn {
    background: linear-gradient(135deg, var(--primary), var(--primary-dark));
    color: white;
    border-radius: 10px;
    font-weight: 500;
    transition: all 0.3s ease;
    position: relative;
    overflow: hidden;
    box-shadow: 
        0 4px 6px -1px rgba(99, 102, 241, 0.3), 
        0 2px 4px -1px rgba(99, 102, 241, 0.2),
        inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.primary-btn::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.3), transparent);
    transition: all 0.6s ease;
}

.primary-btn:hover::before {
    left: 100%;
}

.primary-btn:hover {
    transform: translateY(-2px);
    box-shadow: 
        0 8px 15px -3px rgba(99, 102, 241, 0.4), 
        0 4px 6px -2px rgba(99, 102, 241, 0.3),
        inset 0 1px 0 rgba(255, 255, 255, 0.2);
}

.secondary-btn {
    background: rgba(51, 65, 85, 0.7);
    border: 1px solid rgba(99, 102, 241, 0.3);
    color: var(--light);
    border-radius: 10px;
    font-weight: 500;
    transition: all 0.3s ease;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

.secondary-btn:hover {
    background: rgba(71, 85, 105, 0.8);
    transform: translateY(-2px);
    box-shadow: 
        0 4px 8px -2px rgba(0, 0, 0, 0.2),
        inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.tab-btn {
    position: relative;
    transition: all 0.3s ease;
    font-weight: 500;
    color: var(--gray-light);
    padding: 0.75rem 1rem;
    border-radius: 8px 8px 0 0;
}

.tab-btn.active {
    color: var(--primary-light);
    background: rgba(99, 102, 241, 0.1);
}

.tab-btn.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    width: 100%;
    height: 3px;
    background: linear-gradient(90deg, var(--primary), var(--primary-light));
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.7);
    border-radius: 3px;
}

.glow-text {
    text-shadow: 0 0 10px rgba(99, 102, 241, 0.5);
}

.divider {
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(99, 102, 241, 0.5), transparent);
    margin: 1.5rem 0;
    box-shadow: 0 0 8px rgba(99, 102, 241, 0.3);
}

.glass-code {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid rgba(99, 102, 241, 0.2);
    font-family: 'Courier New', monospace;
    border-radius: 8px;
}

@keyframes pulse {
    0% { opacity: 0.6; }
    50% { opacity: 1; }
    100% { opacity: 0.6; }
}

.pulse-animation {
    animation: pulse 2s infinite ease-in-out;
}

.loading-spinner {
    border: 3px solid rgba(51, 65, 85, 0.3);
    border-top: 3px solid var(--primary);
    border-radius: 50%;
    width: 40px;
    height: 40px;
    animation: spin 1s linear infinite;
}

.loading-spinner-sm {
    border: 2px solid rgba(51, 65, 85, 0.3);
    border-top: 2px solid var(--primary);
    border-radius: 50%;
    width: 24px;
    height: 24px;
    animation: spin 1s linear infinite;
}

.loading-spinner-xs {
    border: 1px solid rgba(51, 65, 85, 0.3);
    border-top: 1px solid var(--primary);
    border-radius: 50%;
    width: 12px;
    height: 12px;
    animation: spin 1s linear infinite;
    display: inline-block;
    vertical-align: middle;
    margin-right: 4px;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

/* Card styles */
.proxy-card {
    background: rgba(30, 41, 59, 0.6);
    border-radius: 16px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    border: 1px solid rgba(99, 102, 241, 0.2);
    transition: all 0.3s ease;
    box-shadow: 
        0 4px 6px -1px rgba(0, 0, 0, 0.1), 
        0 2px 4px -1px rgba(0, 0, 0, 0.06),
        inset 0 1px 0 rgba(255, 255, 255, 0.02);
}

.proxy-card:hover {
    background: rgba(51, 65, 85, 0.7);
    border-color: rgba(99, 102, 241, 0.4);
    transform: translateY(-3px) scale(1.01);
    box-shadow: 
        0 10px 15px -3px rgba(0, 0, 0, 0.2), 
        0 4px 6px -2px rgba(0, 0, 0, 0.1),
        inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

/* Pagination styles */
.pagination-btn {
    min-width: 36px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    font-size: 0.875rem;
    transition: all 0.3s ease;
    background: rgba(51, 65, 85, 0.4);
    border: 1px solid rgba(99, 102, 241, 0.1);
}

.pagination-btn.active {
    background: linear-gradient(135deg, var(--primary), var(--primary-dark));
    color: white;
    border: none;
}

.pagination-btn:not(.active):not(.disabled):hover {
    background: rgba(71, 85, 105, 0.5);
    border-color: rgba(99, 102, 241, 0.3);
}

.pagination-btn.disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Enhanced QR code container */
.qrcode-container {
    background-color: white;
    padding: 15px;
    border-radius: 16px;
    display: inline-block;
    margin: 0 auto;
    box-shadow: 
        0 10px 25px -5px rgba(0, 0, 0, 0.2),
        0 10px 10px -5px rgba(0, 0, 0, 0.1),
        0 0 0 1px rgba(0, 0, 0, 0.05);
    position: relative;
    overflow: hidden;
}

.qrcode-container::before {
    content: '';
    position: absolute;
    top: -50%;
    left: -50%;
    width: 200%;
    height: 200%;
    background: linear-gradient(
        45deg,
        rgba(255, 255, 255, 0.1),
        rgba(255, 255, 255, 0.5),
        rgba(255, 255, 255, 0.1)
    );
    transform: rotate(45deg);
    animation: qr-shimmer 3s linear infinite;
}

@keyframes qr-shimmer {
    0% { transform: translateX(-100%) rotate(45deg); }
    100% { transform: translateX(100%) rotate(45deg); }
}

.qrcode-container img, 
.qrcode-container canvas {
    display: block;
    max-width: 100%;
    height: auto;
}

/* Enhanced footer */
footer {
    position: relative;
    overflow: hidden;
}

footer::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(99, 102, 241, 0.5), transparent);
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.5);
}

/* Info cards */
.info-card {
    background: rgba(15, 23, 42, 0.5);
    border-radius: 8px;
    padding: 0.75rem;
    border: 1px solid rgba(99, 102, 241, 0.1);
}

/* Animated background for header */
.animated-bg {
    position: relative;
    overflow: hidden;
    border-radius: 20px 20px 0 0;
    padding: 2.5rem 0;
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.15), rgba(16, 185, 129, 0.15));
}

.animated-bg::before {
    content: "";
    position: absolute;
    top: -50%;
    left: -50%;
    width: 200%;
    height: 200%;
    background: linear-gradient(
        to bottom right,
        rgba(99, 102, 241, 0),
        rgba(99, 102, 241, 0.2),
        rgba(16, 185, 129, 0.2),
        rgba(16, 185, 129, 0)
    );
    transform: rotate(30deg);
    animation: shimmer 15s linear infinite;
}

@keyframes shimmer {
    0% {
        transform: rotate(30deg) translate(-50%, -50%);
    }
    100% {
        transform: rotate(30deg) translate(50%, 50%);
    }
}

/* Icon styles */
.icon-glow {
    filter: drop-shadow(0 0 8px rgba(99, 102, 241, 0.7));
}

/* Status badge styles */
.status-badge {
    display: inline-flex;
    align-items: center;
    font-size: 0.7rem;
    padding: 0.15rem 0.4rem;
    border-radius: 9999px;
    margin-left: 0.5rem;
    vertical-align: middle;
}

.status-badge.loading {
    background-color: rgba(99, 102, 241, 0.2);
    color: #818cf8;
}

.status-badge.active {
    background-color: rgba(16, 185, 129, 0.2);
    color: #34d399;
}

.status-badge.dead {
    background-color: rgba(239, 68, 68, 0.2);
    color: #f87171;
}

.status-badge.unknown {
    background-color: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
}

/* Responsive adjustments */
@media (max-width: 640px) {
    .tab-btn {
        padding: 0.5rem 0.75rem;
        font-size: 0.75rem;
    }
    
    .tab-btn i {
        margin-right: 0.25rem;
    }
    
    .pagination-btn {
        min-width: 32px;
        height: 32px;
        font-size: 0.75rem;
    }
}

/* Custom Bug dan Wildcard styles */
.wildcard-container {
    display: none;
    margin-top: 0.5rem;
    padding: 0.5rem;
    border-radius: 0.5rem;
    background: rgba(15, 23, 42, 0.3);
}

.wildcard-container.show {
    display: block;
}

.checkbox-container {
    display: flex;
    align-items: center;
}

.checkbox-container input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    margin-right: 0.5rem;
    accent-color: var(--primary);
}

.checkbox-container label {
    font-size: 0.875rem;
    color: var(--gray-light);
}

/* Enhanced Responsive Styles */
@media (max-width: 768px) {
    .container {
        padding-left: 0.75rem;
        padding-right: 0.75rem;
    }
    
    .glass-card {
        border-radius: 12px;
    }
    
    .animated-bg {
        padding: 1.5rem 0;
    }
    
    h1.text-3xl {
        font-size: 1.75rem;
    }
    
    .proxy-card {
        padding: 0.5rem;
    }
    
    .create-account-btn {
        padding: 0.375rem 0.75rem;
        font-size: 0.7rem;
    }
    
    #proxy-basic-info {
        grid-template-columns: 1fr;
    }
    
    .glass-input, .glass-select {
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
    }
    
    .tab-btn {
        padding: 0.5rem;
        font-size: 0.7rem;
    }
    
    .tab-btn i {
        margin-right: 0.25rem;
    }
}

@media (max-width: 480px) {
    .container {
        padding-left: 0.5rem;
        padding-right: 0.5rem;
    }
    
    h1.text-3xl {
        font-size: 1.5rem;
    }
    
    .flex.space-x-3 {
        flex-direction: column;
        gap: 0.5rem;
    }
    
    .flex.space-x-3 > * {
        width: 100%;
    }
    
    .mt-6.flex.space-x-3 {
        flex-direction: row;
    }
    
    .tab-btn {
        padding: 0.375rem;
        font-size: 0.65rem;
    }
    
    .tab-btn i {
        margin-right: 0.125rem;
    }
    
    .qrcode-container {
        padding: 8px;
    }
    
    .qrcode-container canvas,
    .qrcode-container img {
        max-width: 150px;
        height: auto;
    }
    
    footer .flex.flex-wrap {
        gap: 0.5rem;
    }
}

/* Touch-friendly improvements */
@media (hover: none) {
    .primary-btn, .secondary-btn, .pagination-btn, button {
        min-height: 44px; /* Minimum touch target size */
    }
    
    input, select {
        min-height: 44px;
    }
    
    .checkbox-container input[type="checkbox"] {
        width: 1.25rem;
        height: 1.25rem;
    }
}

/* Enhanced info cards */
.info-card {
    background: rgba(15, 23, 42, 0.5);
    border-radius: 8px;
    padding: 0.75rem;
    border: 1px solid rgba(99, 102, 241, 0.1);
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
    position: relative;
    overflow: hidden;
}

.info-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 2px;
    background: linear-gradient(90deg, transparent, rgba(99, 102, 241, 0.3), transparent);
}

/* Enhanced status styles */
.status-active {
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.2), rgba(5, 150, 105, 0.1));
    border: 1px solid rgba(16, 185, 129, 0.3);
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.1);
}

.status-dead {
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.2), rgba(185, 28, 28, 0.1));
    border: 1px solid rgba(239, 68, 68, 0.3);
    box-shadow: 0 0 15px rgba(239, 68, 68, 0.1);
}

.status-unknown {
    background: linear-gradient(135deg, rgba(245, 158, 11, 0.2), rgba(180, 83, 9, 0.1));
    border: 1px solid rgba(245, 158, 11, 0.3);
    box-shadow: 0 0 15px rgba(245, 158, 11, 0.1);
}

/* Donation button and modal styles */
@keyframes spin-slow {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

.animate-spin-slow {
    animation: spin-slow 8s linear infinite;
}

.qris-container {
    box-shadow: 0 0 20px rgba(99, 102, 241, 0.3);
}

#donation-button {
    animation: pulse 2s infinite;
}

@keyframes pulse {
    0% {
        box-shadow: 0 0 0 0 rgba(99, 102, 241, 0.7);
    }
    70% {
        box-shadow: 0 0 0 10px rgba(99, 102, 241, 0);
    }
    100% {
        box-shadow: 0 0 0 0 rgba(99, 102, 241, 0);
    }
}

@media (max-width: 640px) {
    /* Force flex row layout for proxy cards on mobile */
    .proxy-card .flex {
        flex-direction: row !important;
        justify-content: space-between !important;
        align-items: center !important;
    }
    
    /* Adjust proxy info container */
    .proxy-card .flex > div:first-child {
        flex: 1;
        min-width: 0;
        margin-bottom: 0 !important;
        padding-right: 8px;
    }
    
    /* Ensure text truncation for long provider names */
    .proxy-card .font-medium {
        max-width: 150px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        display: block;
    }
    
    /* Make details text smaller on mobile */
    .proxy-card .text-xs {
        font-size: 0.65rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    /* Adjust create button size */
    .create-account-btn {
        padding: 0.375rem 0.5rem !important;
        font-size: 0.65rem !important;
        min-width: 60px !important;
        width: auto !important;
    }
    
    /* Fix status indicator position */
    .proxy-card .flex-items-center {
        display: flex !important;
        align-items: center !important;
        position: relative !important;
    }
    
    /* Ensure status indicator stays in place */
    .proxy-card .inline-block {
        position: static !important;
        margin-left: 4px !important;
        flex-shrink: 0 !important;
    }
}

/* Add floating particles animation */
@keyframes float {
    0% { transform: translateY(0px) rotate(0deg); }
    50% { transform: translateY(-20px) rotate(5deg); }
    100% { transform: translateY(0px) rotate(0deg); }
}

.floating-particle {
    position: absolute;
    opacity: 0.2;
    pointer-events: none;
    z-index: -1;
    animation: float 15s ease-in-out infinite;
}

.floating-particle:nth-child(1) {
    top: 10%;
    left: 10%;
    width: 100px;
    height: 100px;
    background: radial-gradient(circle, rgba(99, 102, 241, 0.7) 0%, rgba(99, 102, 241, 0) 70%);
    border-radius: 50%;
    animation-delay: 0s;
}

.floating-particle:nth-child(2) {
    top: 70%;
    left: 80%;
    width: 150px;
    height: 150px;
    background: radial-gradient(circle, rgba(16, 185, 129, 0.7) 0%, rgba(16, 185, 129, 0) 70%);
    border-radius: 50%;
    animation-delay: -5s;
}

.floating-particle:nth-child(3) {
    top: 40%;
    left: 5%;
    width: 80px;
    height: 80px;
    background: radial-gradient(circle, rgba(244, 63, 94, 0.7) 0%, rgba(244, 63, 94, 0) 70%);
    border-radius: 50%;
    animation-delay: -10s;
}
</style>
</head>
<body class="font-sans">
<div class="container mx-auto px-2 sm:px-4 py-4 sm:py-6 max-w-xl relative">
    <!-- Floating particles -->
    <div class="floating-particle"></div>
    <div class="floating-particle"></div>
    <div class="floating-particle"></div>
    
    <div class="glass-card overflow-hidden">
        <!-- Animated header background -->
        <div class="animated-bg">
            <div class="flex items-center justify-center">
                <div class="mr-3 relative">
                    <div class="absolute inset-0 bg-indigo-500/20 blur-xl rounded-full"></div>
                    <i class="fas fa-network-wired text-4xl icon-glow text-indigo-400 relative z-10"></i>
                </div>
                <h1 class="text-3xl font-bold text-center glow-text bg-clip-text text-transparent bg-gradient-to-r from-indigo-400 via-purple-400 to-emerald-400">Inconigto-Mode</h1>
            </div>
        </div>
        
        <div class="p-6">
            <!-- Proxy List Section -->
            <div id="proxy-list-section">
                <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-5">
                    <div class="flex items-center mb-3 sm:mb-0">
                        <i class="fas fa-server mr-2 text-indigo-400"></i>
                        <h2 class="text-xl font-semibold">Proxy List</h2>
                    </div>
                    <div class="flex flex-col sm:flex-row w-full sm:w-auto space-y-2 sm:space-y-0 sm:space-x-3">
                        <button id="refresh-btn" class="primary-btn py-2 px-4 rounded-lg flex items-center justify-center text-sm">
                            <i class="fas fa-sync-alt mr-2"></i>Refresh
                        </button>
                        <button id="custom-url-btn" class="secondary-btn py-2 px-4 rounded-lg flex items-center justify-center text-sm">
                            <i class="fas fa-link mr-2"></i>URL
                        </button>
                    </div>
                </div>
                
                <div id="custom-url-input" class="mb-5 hidden">
                    <div class="flex">
                        <input type="text" id="proxy-url" class="flex-1 px-4 py-2.5 glass-input rounded-l-lg text-sm" 
                            placeholder="Enter custom proxy list URL" 
                            value="https://raw.githubusercontent.com/InconigtoMode/proxylist/refs/heads/main/all.txt">
                        <button id="load-custom-url" class="primary-btn px-4 py-2.5 rounded-r-lg">
                            Load
                        </button>
                    </div>
                </div>
                
                <div id="loading-indicator" class="text-center py-10 hidden">
                    <div class="loading-spinner mx-auto mb-4"></div>
                    <p class="text-indigo-300 text-sm">Loading proxy list...</p>
                </div>
                
                <!-- Search input -->
                <div class="mb-4">
                    <div class="relative">
                        <input type="text" id="search-input" class="w-full pl-10 pr-4 py-2.5 glass-input rounded-lg text-sm" placeholder="Search by provider or country...">
                        <div class="absolute inset-y-0 left-0 flex items-center pl-3 pointer-events-none">
                            <i class="fas fa-search text-gray-400"></i>
                        </div>
                    </div>
                </div>
                
                <!-- Mobile-friendly proxy list -->
                <div id="proxy-list-container" class="mt-4">
                    <!-- Proxy cards will be populated here -->
                </div>
                
                <!-- Pagination controls -->
                <div id="pagination-container" class="flex justify-center items-center mt-6 space-x-2">
                    <!-- Pagination buttons will be added here -->
                </div>

                <!-- Proxy count info -->
                <div id="proxy-count-info" class="text-center mt-3 text-sm text-gray-400">
                    <!-- Proxy count will be shown here -->
                </div>
                
                <div id="no-proxies-message" class="text-center py-10 hidden">
                    <i class="fas fa-exclamation-circle text-rose-400 text-4xl mb-3"></i>
                    <p class="text-gray-300 text-sm">No proxies found. Please refresh or try a different URL.</p>
                </div>
            </div>
            
            <!-- Account Creation Section -->
            <div id="account-creation-section" class="mt-6 hidden">
                <div class="pt-4">
                    <div class="divider mb-6"></div>
                    <div class="flex justify-between items-center mb-5">
                        <div class="flex items-center">
                            <i class="fas fa-user-plus mr-2 text-indigo-400"></i>
                            <h2 class="text-xl font-semibold">Create Account</h2>
                        </div>
                        <button id="back-to-list" class="flex items-center text-indigo-400 hover:text-indigo-300 transition-colors text-sm">
                            <i class="fas fa-arrow-left mr-2"></i> Back to List
                        </button>
                    </div>
                    
                    <div class="glass-card p-5 mb-6 overflow-hidden relative">
    <div class="absolute top-0 right-0 w-32 h-32 bg-gradient-to-bl from-indigo-500/5 to-transparent rounded-bl-full pointer-events-none"></div>
    
    <h3 class="text-base font-medium text-indigo-400 mb-4 flex items-center">
        <i class="fas fa-server mr-2"></i>
        Selected Proxy
    </h3>
    
    <div class="grid grid-cols-2 gap-3 mb-4">
        <div class="flex items-center space-x-2 bg-slate-800/40 p-3 rounded-lg border border-slate-700/30">
            <i class="fas fa-building text-indigo-400/70 w-5 text-center"></i>
            <div>
                <div class="text-xs text-gray-400">Provider</div>
                <div id="selected-provider" class="text-sm font-medium text-white">-</div>
            </div>
        </div>
        <div class="flex items-center space-x-2 bg-slate-800/40 p-3 rounded-lg border border-slate-700/30">
            <i class="fas fa-globe-americas text-indigo-400/70 w-5 text-center"></i>
            <div>
                <div class="text-xs text-gray-400">Country</div>
                <div id="selected-country" class="text-sm font-medium text-white">-</div>
            </div>
        </div>
        <div class="flex items-center space-x-2 bg-slate-800/40 p-3 rounded-lg border border-slate-700/30">
            <i class="fas fa-network-wired text-indigo-400/70 w-5 text-center"></i>
            <div>
                <div class="text-xs text-gray-400">IP</div>
                <div id="selected-ip" class="text-sm font-medium text-white">-</div>
            </div>
        </div>
        <div class="flex items-center space-x-2 bg-slate-800/40 p-3 rounded-lg border border-slate-700/30">
            <i class="fas fa-plug text-indigo-400/70 w-5 text-center"></i>
            <div>
                <div class="text-xs text-gray-400">Port</div>
                <div id="selected-port" class="text-sm font-medium text-white">-</div>
            </div>
        </div>
    </div>
    
    <div id="proxy-status-container" class="hidden">
        <div id="proxy-status-active" class="hidden rounded-lg p-3 bg-gradient-to-r from-emerald-900/20 to-emerald-800/10 border border-emerald-500/20">
            <div class="flex items-center">
                <div class="w-10 h-10 rounded-full bg-emerald-500/10 flex items-center justify-center mr-3 border border-emerald-500/30">
                    <i class="fas fa-check text-emerald-400"></i>
                </div>
                <div>
                    <div class="text-sm font-medium text-emerald-400">ACTIVE</div>
                    <div class="text-xs text-gray-400">Latency: <span id="proxy-latency" class="text-amber-400">0ms</span></div>
                </div>
            </div>
        </div>
        
        <div id="proxy-status-dead" class="hidden rounded-lg p-3 bg-gradient-to-r from-rose-900/20 to-rose-800/10 border border-rose-500/20">
            <div class="flex items-center">
                <div class="w-10 h-10 rounded-full bg-rose-500/10 flex items-center justify-center mr-3 border border-rose-500/30">
                    <i class="fas fa-times text-rose-500"></i>
                </div>
                <div>
                    <div class="text-sm font-medium text-rose-500">DEAD</div>
                    <div class="text-xs text-gray-400">This proxy may not be working</div>
                </div>
            </div>
        </div>
        
        <div id="proxy-status-unknown" class="hidden rounded-lg p-3 bg-gradient-to-r from-amber-900/20 to-amber-800/10 border border-amber-500/20">
            <div class="flex items-center">
                <div class="w-10 h-10 rounded-full bg-amber-500/10 flex items-center justify-center mr-3 border border-amber-500/30">
                    <i class="fas fa-question text-amber-400"></i>
                </div>
                <div>
                    <div class="text-sm font-medium text-amber-400">UNKNOWN</div>
                    <div class="text-xs text-gray-400">Could not check proxy status</div>
                </div>
            </div>
        </div>
        
        
<div id="proxy-status-loading" class="rounded-lg p-3 bg-gradient-to-r from-indigo-900/20 to-indigo-800/10 border border-indigo-500/20">
    <div class="flex items-center">
        <div class="w-10 h-10 rounded-full bg-indigo-500/10 flex items-center justify-center mr-3 border border-indigo-500/30">
            <div class="loading-spinner-sm"></div>
        </div>
        <div>
            <div class="text-sm font-medium text-indigo-400">CHECKING</div>
            <div class="text-xs text-gray-400">Verifying proxy status...</div>
        </div>
    </div>
</div>
    </div>
</div>
                    
                    <!-- Tabs -->
                    <div class="flex border-b border-gray-700 mb-6 overflow-x-auto space-x-1 sm:space-x-0">
                        <button class="tab-btn active py-3 px-4 font-medium text-sm focus:outline-none" data-target="vmess-form">
                            <i class="fas fa-shield-alt mr-2"></i>VMess
                        </button>
                        <button class="tab-btn py-3 px-4 font-medium text-sm focus:outline-none" data-target="vless-form">
                            <i class="fas fa-bolt mr-2"></i>VLESS
                        </button>
                        <button class="tab-btn py-3 px-4 font-medium text-sm focus:outline-none" data-target="trojan-form">
                            <i class="fas fa-user-secret mr-2"></i>Trojan
                        </button>
                        <button class="tab-btn py-3 px-4 font-medium text-sm focus:outline-none" data-target="ss-form">
                            <i class="fas fa-mask mr-2"></i>SS
                        </button>
                    </div>
                    
                    <!-- VMess Form -->
                    <div id="vmess-form" class="protocol-form">
                        <form id="vmess-account-form" class="space-y-5">
                            <div>
                                <label for="vmess-name" class="block text-sm font-medium text-gray-300 mb-2">Account Name</label>
                                <input type="text" id="vmess-name" name="name" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="vmess-uuid" class="block text-sm font-medium text-gray-300 mb-2">UUID</label>
                                <div class="flex">
                                    <input type="text" id="vmess-uuid" name="uuid" class="flex-1 px-4 py-2.5 glass-input rounded-l-lg text-sm" value="bbbbbbbb-cccc-4ddd-eeee-ffffffffffff" required>
                                    <button type="button" onclick="generateUUID('vmess-uuid')" class="secondary-btn px-4 py-2.5 rounded-r-lg hover:bg-slate-700">
                                        <i class="fas fa-sync-alt"></i>
                                    </button>
                                </div>
                            </div>
                            
                            <div>
                                <label for="vmess-path" class="block text-sm font-medium text-gray-300 mb-2">Path</label>
                                <input type="text" id="vmess-path" name="path" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="vmess-security" class="block text-sm font-medium text-gray-300 mb-2">TLS</label>
                                <select id="vmess-security" name="security" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <option value="tls">TLS</option>
                                    <option value="none">None</option>
                                </select>
                                <input type="hidden" id="vmess-encryption" name="encryption" value="zero">
                            </div>
                            
                            <div>
                                <label for="vmess-server-domain" class="block text-sm font-medium text-gray-300 mb-2">Server Domain</label>
                                <select id="vmess-server-domain" name="server-domain" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <!-- Options will be populated dynamically -->
                                </select>
                            </div>
                            
                            <div>
                                <label for="vmess-bug" class="block text-sm font-medium text-gray-300 mb-2">Custom Bug <span class="text-xs text-gray-500">(opsional)</span></label>
                                <input type="text" id="vmess-bug" name="bug" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" placeholder="e.g. bug.com">
                                <div id="vmess-wildcard-container" class="wildcard-container">
                                    <div class="checkbox-container">
                                        <input type="checkbox" id="vmess-wildcard" name="wildcard">
                                        <label for="vmess-wildcard">Gunakan Wildcard (bug.domain.com)</label>
                                    </div>
                                </div>
                            </div>
                            
                            <div class="pt-2">
                                <button type="submit" class="w-full primary-btn py-3 px-4 rounded-lg flex items-center justify-center">
                                    <i class="fas fa-plus-circle mr-2"></i> Create VMess Account
                                </button>
                            </div>
                        </form>
                    </div>
                    
                    <!-- VLESS Form -->
                    <div id="vless-form" class="protocol-form hidden">
                        <form id="vless-account-form" class="space-y-5">
                            <div>
                                <label for="vless-name" class="block text-sm font-medium text-gray-300 mb-2">Account Name</label>
                                <input type="text" id="vless-name" name="name" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="vless-uuid" class="block text-sm font-medium text-gray-300 mb-2">UUID</label>
                                <div class="flex">
                                    <input type="text" id="vless-uuid" name="uuid" class="flex-1 px-4 py-2.5 glass-input rounded-l-lg text-sm" value="bbbbbbbb-cccc-4ddd-eeee-ffffffffffff" required>
                                    <button type="button" onclick="generateUUID('vless-uuid')" class="secondary-btn px-4 py-2.5 rounded-r-lg hover:bg-slate-700">
                                        <i class="fas fa-sync-alt"></i>
                                    </button>
                                </div>
                            </div>
                            
                            <div>
                                <label for="vless-path" class="block text-sm font-medium text-gray-300 mb-2">Path</label>
                                <input type="text" id="vless-path" name="path" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="vless-security" class="block text-sm font-medium text-gray-300 mb-2">TLS</label>
                                <select id="vless-security" name="security" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <option value="tls">TLS</option>
                                    <option value="none">None</option>
                                </select>
                                <input type="hidden" id="vless-encryption" name="encryption" value="none">
                            </div>
                            
                            <div>
                                <label for="vless-server-domain" class="block text-sm font-medium text-gray-300 mb-2">Server Domain</label>
                                <select id="vless-server-domain" name="server-domain" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <!-- Options will be populated dynamically -->
                                </select>
                            </div>
                            
                            <div>
                                <label for="vless-bug" class="block text-sm font-medium text-gray-300 mb-2">Custom Bug <span class="text-xs text-gray-500">(opsional)</span></label>
                                <input type="text" id="vless-bug" name="bug" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" placeholder="e.g. bug.com">
                                <div id="vless-wildcard-container" class="wildcard-container">
                                    <div class="checkbox-container">
                                        <input type="checkbox" id="vless-wildcard" name="wildcard">
                                        <label for="vless-wildcard">Gunakan Wildcard (bug.domain.com)</label>
                                    </div>
                                </div>
                            </div>
                            
                            <div class="pt-2">
                                <button type="submit" class="w-full primary-btn py-3 px-4 rounded-lg flex items-center justify-center">
                                    <i class="fas fa-plus-circle mr-2"></i> Create VLESS Account
                                </button>
                            </div>
                        </form>
                    </div>
                    
                    <!-- Trojan Form -->
                    <div id="trojan-form" class="protocol-form hidden">
                        <form id="trojan-account-form" class="space-y-5">
                            <div>
                                <label for="trojan-name" class="block text-sm font-medium text-gray-300 mb-2">Account Name</label>
                                <input type="text" id="trojan-name" name="name" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="trojan-password" class="block text-sm font-medium text-gray-300 mb-2">Password</label>
                                <div class="flex">
                                    <input type="text" id="trojan-password" name="password" class="flex-1 px-4 py-2.5 glass-input rounded-l-lg text-sm" value="bbbbbbbb-cccc-4ddd-eeee-ffffffffffff" required>
                                    <button type="button" onclick="generatePassword('trojan-password')" class="secondary-btn px-4 py-2.5 rounded-r-lg hover:bg-slate-700">
                                        <i class="fas fa-sync-alt"></i>
                                    </button>
                                </div>
                            </div>
                            
                            <div>
                                <label for="trojan-path" class="block text-sm font-medium text-gray-300 mb-2">Path</label>
                                <input type="text" id="trojan-path" name="path" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="trojan-security" class="block text-sm font-medium text-gray-300 mb-2">TLS</label>
                                <select id="trojan-security" name="security" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <option value="tls">TLS</option>
                                    <option value="none">None</option>
                                </select>
                                <input type="hidden" id="trojan-sni" name="sni" value="inconigto-mode.web.id">
                            </div>
                            
                            <div>
                                <label for="trojan-server-domain" class="block text-sm font-medium text-gray-300 mb-2">Server Domain</label>
                                <select id="trojan-server-domain" name="server-domain" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <!-- Options will be populated dynamically -->
                                </select>
                            </div>
                            
                            <div>
                                <label for="trojan-bug" class="block text-sm font-medium text-gray-300 mb-2">Custom Bug <span class="text-xs text-gray-500">(opsional)</span></label>
                                <input type="text" id="trojan-bug" name="bug" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" placeholder="e.g. bug.com">
                                <div id="trojan-wildcard-container" class="wildcard-container">
                                    <div class="checkbox-container">
                                        <input type="checkbox" id="trojan-wildcard" name="wildcard">
                                        <label for="trojan-wildcard">Gunakan Wildcard (bug.domain.com)</label>
                                    </div>
                                </div>
                            </div>
                            
                            <div class="pt-2">
                                <button type="submit" class="w-full primary-btn py-3 px-4 rounded-lg flex items-center justify-center">
                                    <i class="fas fa-plus-circle mr-2"></i> Create Trojan Account
                                </button>
                            </div>
                        </form>
                    </div>
                    
                    <!-- Shadowsocks Form -->
                    <div id="ss-form" class="protocol-form hidden">
                        <form id="ss-account-form" class="space-y-5">
                            <div>
                                <label for="ss-name" class="block text-sm font-medium text-gray-300 mb-2">Account Name</label>
                                <input type="text" id="ss-name" name="name" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="ss-password" class="block text-sm font-medium text-gray-300 mb-2">Password</label>
                                <div class="flex">
                                    <input type="text" id="ss-password" name="password" class="flex-1 px-4 py-2.5 glass-input rounded-l-lg text-sm" value="bbbbbbbb-cccc-4ddd-eeee-ffffffffffff" required>
                                    <button type="button" onclick="generatePassword('ss-password')" class="secondary-btn px-4 py-2.5 rounded-r-lg hover:bg-slate-700">
                                        <i class="fas fa-sync-alt"></i>
                                    </button>
                                </div>
                            </div>
                            
                            <div>
                                <label for="ss-path" class="block text-sm font-medium text-gray-300 mb-2">Path</label>
                                <input type="text" id="ss-path" name="path" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" readonly>
                            </div>
                            
                            <div>
                                <label for="ss-security" class="block text-sm font-medium text-gray-300 mb-2">TLS</label>
                                <select id="ss-security" name="security" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <option value="tls">TLS</option>
                                    <option value="none">None</option>
                                </select>
                            </div>
                            
                            <div>
                                <label for="ss-server-domain" class="block text-sm font-medium text-gray-300 mb-2">Server Domain</label>
                                <select id="ss-server-domain" name="server-domain" class="w-full px-4 py-2.5 glass-select rounded-lg text-sm">
                                    <!-- Options will be populated dynamically -->
                                </select>
                            </div>
                            
                            <div>
                                <label for="ss-bug" class="block text-sm font-medium text-gray-300 mb-2">Custom Bug <span class="text-xs text-gray-500">(opsional)</span></label>
                                <input type="text" id="ss-bug" name="bug" class="w-full px-4 py-2.5 glass-input rounded-lg text-sm" placeholder="e.g. bug.com">
                                <div id="ss-wildcard-container" class="wildcard-container">
                                    <div class="checkbox-container">
                                        <input type="checkbox" id="ss-wildcard" name="wildcard">
                                        <label for="ss-wildcard">Gunakan Wildcard (bug.domain.com)</label>
                                    </div>
                                </div>
                            </div>
                            
                            <div class="pt-2">
                                <button type="submit" class="w-full primary-btn py-3 px-4 rounded-lg flex items-center justify-center">
                                    <i class="fas fa-plus-circle mr-2"></i> Create Shadowsocks Account
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
            
            <!-- Result Section -->
            <div id="result-section" class="mt-6 hidden">
                <div class="pt-4">
                    <div class="divider mb-6"></div>
                    <div class="flex justify-between items-center mb-5">
                        <div class="flex items-center">
                            <i class="fas fa-check-circle mr-2 text-emerald-400"></i>
                            <h2 class="text-xl font-semibold">Account Created</h2>
                        </div>
                        <button id="back-to-form" class="flex items-center text-indigo-400 hover:text-indigo-300 transition-colors text-sm">
                            <i class="fas fa-arrow-left mr-2"></i> Back
                        </button>
                    </div>
                    
                    <div class="glass-card p-5 mb-5">
                        <div class="flex justify-between items-center mb-3">
                            <h3 class="text-base font-medium text-indigo-400">Connection URL</h3>
                            <button id="copy-url" class="flex items-center text-indigo-400 hover:text-indigo-300 text-xs transition-colors">
                                <i class="far fa-copy mr-1"></i> Copy
                            </button>
                        </div>
                        <div id="connection-url" class="text-xs glass-code p-4 rounded-lg break-all font-mono"></div>
                    </div>
                    
                    <div class="glass-card p-5">
                        <div class="flex justify-between items-center mb-3">
                            <h3 class="text-base font-medium text-indigo-400">QR Code</h3>
                            <button id="download-qr" class="flex items-center text-indigo-400 hover:text-indigo-300 text-xs transition-colors">
                                <i class="fas fa-download mr-1"></i> Download
                            </button>
                        </div>
                        <div class="flex justify-center py-4">
                            <div id="qrcode" class="qrcode-container"></div>
                        </div>
                    </div>
                    
                    <div class="mt-6 flex flex-col sm:flex-row space-y-3 sm:space-y-0 sm:space-x-3">
                        <button id="create-new" class="flex-1 primary-btn py-3 px-4 rounded-lg flex items-center justify-center text-sm">
                            <i class="fas fa-plus-circle mr-2"></i> Create Another
                        </button>
                        <button id="back-to-list-from-result" class="flex-1 secondary-btn py-3 px-4 rounded-lg flex items-center justify-center text-sm">
                            <i class="fas fa-list mr-2"></i> Back to List
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <footer class="mt-8 pb-6">
    <div class="divider mb-6"></div>
    <div class="relative overflow-hidden rounded-xl p-6 bg-slate-800/30 backdrop-blur-sm border border-slate-700/30">
        <div class="absolute top-0 right-0 w-40 h-40 bg-indigo-500/5 rounded-bl-full pointer-events-none"></div>
        <div class="absolute bottom-0 left-0 w-40 h-40 bg-emerald-500/5 rounded-tr-full pointer-events-none"></div>
        
        <div class="flex flex-col md:flex-row justify-between items-center gap-6">
            <div class="flex items-center">
                <div class="w-10 h-10 rounded-full bg-indigo-500/10 flex items-center justify-center mr-3 border border-indigo-500/20">
                    <i class="fas fa-network-wired text-lg icon-glow text-indigo-400"></i>
                </div>
                <span class="text-lg font-semibold bg-clip-text text-transparent bg-gradient-to-r from-indigo-400 to-emerald-400">Inconigto-Mode</span>
            </div>
            
            
            <div class="flex gap-4">
                <a href="#" class="w-8 h-8 rounded-full bg-slate-700/50 flex items-center justify-center text-gray-400 hover:text-indigo-400 hover:bg-slate-700 transition-all">
                    <i class="fab fa-github"></i>
                </a>
                <a href="https://t.me/InconigtoMode" class="w-8 h-8 rounded-full bg-slate-700/50 flex items-center justify-center text-gray-400 hover:text-indigo-400 hover:bg-slate-700 transition-all">
                    <i class="fab fa-telegram"></i>
                </a>
                <a href="https://t.me/Noir7R" class="w-8 h-8 rounded-full bg-slate-700/50 flex items-center justify-center text-gray-400 hover:text-indigo-400 hover:bg-slate-700 transition-all">
                    <i class="fab fa-telegram"></i>
                </a>
            </div>
        </div>
        
        <div class="h-px bg-gradient-to-r from-transparent via-slate-600/20 to-transparent my-5"></div>
        
        <div class="text-center text-gray-500 text-xs">
            <p>© 2025 Inconigto-Mode | Secure Connection Technology</p>
            <p class="mt-1 flex items-center justify-center">
                Dibuat dengan 
                <i class="fas fa-heart text-rose-500 mx-1 animate-pulse"></i> 
                untuk privasi online Anda
            </p>
        </div>
    </div>
</footer>
</div>

<div id="donation-button" class="fixed bottom-6 right-6 z-[1000]">
    <button class="w-14 h-14 rounded-full bg-gradient-to-r from-indigo-500 to-purple-600 flex items-center justify-center shadow-lg hover:shadow-xl transform hover:scale-110 transition-all duration-300 pulse-animation relative">
        <div class="absolute inset-0 rounded-full bg-gradient-to-r from-indigo-500 to-purple-600 blur-md opacity-70"></div>
        <i class="fas fa-hand-holding-heart text-white text-xl relative z-10"></i>
    </button>
</div>

<!-- QR Code Donation Modal -->
<div id="donation-modal" class="fixed inset-0 z-[1001] flex items-center justify-center p-4 opacity-0 pointer-events-none transition-opacity duration-300">
    <div class="absolute inset-0 bg-black bg-opacity-70 backdrop-blur-sm" id="donation-backdrop"></div>
    <div class="relative bg-gradient-to-br from-slate-900 to-slate-800 rounded-2xl p-1 max-w-md w-full transform scale-95 transition-all duration-300" id="donation-content">
        <!-- Animated border -->
        <div class="absolute inset-0 rounded-2xl overflow-hidden">
            <div class="absolute inset-0 bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500 animate-spin-slow opacity-70"></div>
        </div>
        
        <div class="relative bg-slate-900 rounded-xl p-6 flex flex-col items-center">
            <button id="close-donation" class="absolute top-2 right-2 w-8 h-8 flex items-center justify-center rounded-full bg-slate-800 hover:bg-slate-700 text-gray-400 hover:text-white transition-colors">
                <i class="fas fa-times"></i>
            </button>
            
            <h3 class="text-xl font-bold mb-2 text-center bg-clip-text text-transparent bg-gradient-to-r from-indigo-400 to-purple-400">Support Inconigto-Mode</h3>
            <p class="text-gray-400 text-sm mb-4 text-center">Your donation helps keep our services running</p>
            
            <div class="qris-container p-3 bg-white rounded-xl mb-4 relative overflow-hidden">
                <!-- Inner animated border -->
                <div class="absolute inset-0 p-2">
                    <div class="absolute inset-0 bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500 animate-pulse rounded-lg opacity-70"></div>
                </div>
                <div class="relative bg-white p-2 rounded-lg">
                    <img src="https://raw.githubusercontent.com/InconigtoMode/proxylist/refs/heads/main/qris.png" alt="Donation QR Code" class="w-full h-auto max-w-xs mx-auto">
                </div>
            </div>
            
            <p class="text-gray-400 text-xs text-center">Scan this QR code with your payment app to donate</p>
        </div>
    </div>
</div>

<script>
// Global variables
let proxyList = [];
let filteredProxyList = [];
let selectedProxy = null;
const defaultProxyUrl = 'https://raw.githubusercontent.com/InconigtoMode/proxylist/refs/heads/main/all.txt';
// Change from:
// const serverDomain = 'inconigto-mode.web.id';

// To:
const serverDomains = ['inconigto-mode.web.id', 'inconigto-mode.biz.id'];
let selectedServerDomain = serverDomains[0]; // Default to first domain
const defaultUUID = 'bbbbbbbb-cccc-4ddd-eeee-ffffffffffff';
const itemsPerPage = 10;
let currentPage = 1;

// DOM elements
const proxyListSection = document.getElementById('proxy-list-section');
const accountCreationSection = document.getElementById('account-creation-section');
const resultSection = document.getElementById('result-section');
const loadingIndicator = document.getElementById('loading-indicator');
const proxyListContainer = document.getElementById('proxy-list-container');
const noProxiesMessage = document.getElementById('no-proxies-message');
const customUrlInput = document.getElementById('custom-url-input');
const proxyUrlInput = document.getElementById('proxy-url');
const paginationContainer = document.getElementById('pagination-container');
const proxyCountInfo = document.getElementById('proxy-count-info');
const searchInput = document.getElementById('search-input');

// Initialize
document.addEventListener('DOMContentLoaded', function() {
    // Display fallback proxy list immediately to ensure something is visible
    displayFallbackProxyList();
    
    // Then try to load the actual proxy list
    loadProxyList(defaultProxyUrl);
    
    // Event listeners
    document.getElementById('refresh-btn').addEventListener('click', function() {
        loadProxyList(defaultProxyUrl);
    });
    
    document.getElementById('custom-url-btn').addEventListener('click', function() {
        customUrlInput.classList.toggle('hidden');
    });
    
    document.getElementById('load-custom-url').addEventListener('click', function() {
        const url = proxyUrlInput.value.trim();
        if (url) {
            loadProxyList(url);
        }
    });
    
    document.getElementById('back-to-list').addEventListener('click', function() {
        showProxyListSection();
    });
    
    document.getElementById('back-to-form').addEventListener('click', function() {
        resultSection.classList.add('hidden');
        accountCreationSection.classList.remove('hidden');
    });
    
    document.getElementById('create-new').addEventListener('click', function() {
        resultSection.classList.add('hidden');
        accountCreationSection.classList.remove('hidden');
    });
    
    document.getElementById('back-to-list-from-result').addEventListener('click', function() {
        showProxyListSection();
    });
    
    // Search functionality
    searchInput.addEventListener('input', function() {
        const searchTerm = this.value.toLowerCase().trim();
        
        if (searchTerm === '') {
            filteredProxyList = [...proxyList];
        } else {
            filteredProxyList = proxyList.filter(proxy => 
                proxy.provider.toLowerCase().includes(searchTerm) || 
                proxy.country.toLowerCase().includes(searchTerm)
            );
        }
        
        currentPage = 1;
        renderProxyList();
    });
    
    // Protocol tabs
    const protocolTabs = document.querySelectorAll('.tab-btn');
    const protocolForms = document.querySelectorAll('.protocol-form');
    
    protocolTabs.forEach(tab => {
        tab.addEventListener('click', () => {
            // Remove active class from all tabs
            protocolTabs.forEach(t => {
                t.classList.remove('active');
            });
            
            // Add active class to clicked tab
            tab.classList.add('active');
            
            // Hide all forms
            protocolForms.forEach(form => {
                form.classList.add('hidden');
            });
            
            // Show the selected form
            const targetId = tab.getAttribute('data-target');
            document.getElementById(targetId).classList.remove('hidden');
        });
    });
    
    // Populate server domain dropdowns
    const serverDomainSelects = [
        document.getElementById('vmess-server-domain'),
        document.getElementById('vless-server-domain'),
        document.getElementById('trojan-server-domain'),
        document.getElementById('ss-server-domain')
    ];

    serverDomainSelects.forEach(select => {
        if (select) {
            // Clear existing options
            select.innerHTML = '';
            
            // Add options for each domain
            serverDomains.forEach(domain => {
                const option = document.createElement('option');
                option.value = domain;
                option.textContent = domain;
                select.appendChild(option);
            });
            
            // Add event listener to update selected domain
            select.addEventListener('change', function() {
                selectedServerDomain = this.value;
            });
        }
    });
    
    // Form submissions
    const forms = [
        document.getElementById('vmess-account-form'),
        document.getElementById('vless-account-form'),
        document.getElementById('trojan-account-form'),
        document.getElementById('ss-account-form')
    ];
    
    // Custom Bug dan Wildcard functionality
    const bugInputs = [
        document.getElementById('vmess-bug'),
        document.getElementById('vless-bug'),
        document.getElementById('trojan-bug'),
        document.getElementById('ss-bug')
    ];

    const wildcardContainers = [
        document.getElementById('vmess-wildcard-container'),
        document.getElementById('vless-wildcard-container'),
        document.getElementById('trojan-wildcard-container'),
        document.getElementById('ss-wildcard-container')
    ];

    const wildcardCheckboxes = [
        document.getElementById('vmess-wildcard'),
        document.getElementById('vless-wildcard'),
        document.getElementById('trojan-wildcard'),
        document.getElementById('ss-wildcard')
    ];

    // Add event listeners to bug inputs
    bugInputs.forEach((input, index) => {
        input.addEventListener('input', function() {
            if (this.value.trim() !== '') {
                wildcardContainers[index].classList.add('show');
            } else {
                wildcardContainers[index].classList.remove('show');
                wildcardCheckboxes[index].checked = false;
            }
        });
    });

    forms.forEach(form => {
        form.addEventListener('submit', function(e) {
            e.preventDefault();
            
            // Get form data
            const formData = new FormData(form);
            const formType = form.id.split('-')[0]; // vmess, vless, trojan, or ss
            
            // Get custom bug and wildcard values
            const customBug = formData.get('bug') ? formData.get('bug').toString().trim() : '';
            const useWildcard = formData.get('wildcard') === 'on';
            
            // Determine server, host, and SNI based on custom bug and wildcard
            // Get the selected server domain from the form
            const selectedDomain = formData.get('server-domain') || selectedServerDomain;
            let server = selectedDomain;
            let host = selectedDomain;
            let sni = selectedDomain;
            
            if (customBug) {
                server = customBug;
                if (useWildcard) {
                    host = `${customBug}.${selectedDomain}`;
                    sni = `${customBug}.${selectedDomain}`;
                }
            }
            
            // Generate connection URL based on protocol
            let connectionUrl = '';
            
            if (formType === 'vmess') {
                const security = formData.get('security');
                // Set port based on TLS setting
                const port = security === 'tls' ? 443 : 80;
                
                const vmessConfig = {
                    v: '2',
                    ps: formData.get('name'),
                    add: server,
                    port: port,
                    id: formData.get('uuid'),
                    aid: '0',
                    net: 'ws', // Always WebSocket
                    type: 'none',
                    host: host,
                    path: formData.get('path'),
                    tls: security === 'tls' ? 'tls' : '',
                    sni: sni,
                    scy: 'zero'
                };
                
                connectionUrl = 'vmess://' + btoa(JSON.stringify(vmessConfig));
            } else if (formType === 'vless') {
                const uuid = formData.get('uuid');
                const path = encodeURIComponent(formData.get('path'));
                const security = formData.get('security');
                const encryption = 'none';
                const name = encodeURIComponent(formData.get('name'));
                // Set port based on TLS setting
                const port = security === 'tls' ? 443 : 80;
                
                connectionUrl = `vless://${uuid}@${server}:${port}?encryption=${encryption}&security=${security}&type=ws&host=${host}&path=${path}&sni=${sni}#${name}`;
            } else if (formType === 'trojan') {
                const password = formData.get('password');
                const path = encodeURIComponent(formData.get('path'));
                const security = formData.get('security');
                const name = encodeURIComponent(formData.get('name'));
                // Set port based on TLS setting
                const port = security === 'tls' ? 443 : 80;
                
                connectionUrl = `trojan://${password}@${server}:${port}?security=${security}&type=ws&host=${host}&path=${path}&sni=${sni}#${name}`;
            } else if (formType === 'ss') {
                const password = formData.get('password');
                const name = encodeURIComponent(formData.get('name'));
                const path = encodeURIComponent(formData.get('path'));
                const security = formData.get('security');
                
                // Set port based on TLS setting
                const port = security === 'tls' ? 443 : 80;
                
                // Use fixed cipher: none for Shadowsocks
                const method = "none";
                
                // Base64 encode the method:password part
                const userInfo = btoa(`${method}:${password}`);
                
                // Create the new format SS URL with dynamic port
                connectionUrl = `ss://${userInfo}@${server}:${port}?encryption=none&type=ws&host=${host}&path=${path}&security=${security}&sni=${sni}#${name}`;
            }
            
            // Display the result
            document.getElementById('connection-url').textContent = connectionUrl;
            
            // Generate QR code - Improved with multiple fallback methods
            generateQRCode(connectionUrl);
            
            // Show result section
            accountCreationSection.classList.add('hidden');
            resultSection.classList.remove('hidden');
        });
    });
    
    // Copy URL button
    document.getElementById('copy-url').addEventListener('click', function() {
        const connectionUrl = document.getElementById('connection-url').textContent;
        navigator.clipboard.writeText(connectionUrl).then(() => {
            this.innerHTML = '<i class="fas fa-check mr-1"></i> Copied!';
            setTimeout(() => {
                this.innerHTML = '<i class="far fa-copy mr-1"></i> Copy';
            }, 2000);
        });
    });
    
    // Download QR code button
    document.getElementById('download-qr').addEventListener('click', function() {
        downloadQRCode();
    });
    
    // Donation modal functionality
    const donationButton = document.getElementById('donation-button');
    const donationModal = document.getElementById('donation-modal');
    const donationContent = document.getElementById('donation-content');
    const donationBackdrop = document.getElementById('donation-backdrop');
    const closeButton = document.getElementById('close-donation');
    
    function openDonationModal() {
        // Show the modal
        donationModal.classList.remove('opacity-0', 'pointer-events-none');
        donationModal.classList.add('opacity-100');
        
        // Animate the content
        setTimeout(() => {
            donationContent.classList.remove('scale-95');
            donationContent.classList.add('scale-100');
        }, 10);
    }
    
    function closeDonationModal() {
        // Animate the content
        donationContent.classList.remove('scale-100');
        donationContent.classList.add('scale-95');
        
        // Hide the modal
        setTimeout(() => {
            donationModal.classList.remove('opacity-100');
            donationModal.classList.add('opacity-0', 'pointer-events-none');
        }, 200);
    }
    
    // Event listeners
    donationButton.addEventListener('click', openDonationModal);
    closeButton.addEventListener('click', closeDonationModal);
    donationBackdrop.addEventListener('click', closeDonationModal);
});

// Improved QR code generation with multiple fallback methods
function generateQRCode(text) {
    const qrcodeElement = document.getElementById('qrcode');
    qrcodeElement.innerHTML = '';
    
    // Try multiple methods to generate QR code
    try {
        // Method 1: Try to generate QR code using toCanvas
        QRCode.toCanvas(qrcodeElement, text, { 
            width: 200,
            margin: 1,
            color: {
                dark: '#000000',
                light: '#FFFFFF'
            }
        }, function(error) {
            if (error) {
                console.error("QR Code canvas error:", error);
                // If canvas fails, try method 2
                generateQRCodeFallback(text, qrcodeElement);
            }
        });
    } catch (error) {
        console.error("QR Code generation error:", error);
        // If method 1 fails completely, try method 2
        generateQRCodeFallback(text, qrcodeElement);
    }
}

// Fallback QR code generation method
function generateQRCodeFallback(text, container) {
    try {
        // Method 2: Try to generate QR code as SVG
        QRCode.toString(text, {
            type: 'svg',
            width: 200,
            margin: 1,
            color: {
                dark: '#000000',
                light: '#FFFFFF'
            }
        }, function(error, svg) {
            if (error || !svg) {
                console.error("QR Code SVG error:", error);
                // If SVG fails, try method 3
                generateQRCodeLastResort(text, container);
            } else {
                container.innerHTML = svg;
            }
        });
    } catch (error) {
        console.error("QR Code SVG generation error:", error);
        // If method 2 fails completely, try method 3
        generateQRCodeLastResort(text, container);
    }
}

// Last resort QR code generation method
function generateQRCodeLastResort(text, container) {
    try {
        // Method 3: Try to generate QR code as data URL
        const encodedText = encodeURIComponent(text);
        const qrApiUrl = `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodedText}`;
        
        const img = document.createElement('img');
        img.src = qrApiUrl;
        img.alt = "QR Code";
        img.width = 200;
        img.height = 200;
        img.onerror = function() {
            container.innerHTML = '<div class="text-center text-rose-500">Failed to generate QR code</div>';
        };
        
        container.innerHTML = '';
        container.appendChild(img);
    } catch (error) {
        console.error("QR Code last resort error:", error);
        container.innerHTML = '<div class="text-center text-rose-500">Failed to generate QR code</div>';
    }
}

// Download QR code function
function downloadQRCode() {
    const qrcodeElement = document.getElementById('qrcode');
    
    // Try to find canvas or img in the QR code container
    const canvas = qrcodeElement.querySelector('canvas');
    const img = qrcodeElement.querySelector('img');
    const svg = qrcodeElement.querySelector('svg');
    
    let imageUrl = null;
    
    if (canvas) {
        // If canvas exists, convert it to data URL
        try {
            imageUrl = canvas.toDataURL('image/png');
        } catch (e) {
            console.error("Canvas to data URL error:", e);
        }
    } else if (img) {
        // If img exists, use its src
        imageUrl = img.src;
    } else if (svg) {
        // If SVG exists, convert it to data URL
        try {
            const svgData = new XMLSerializer().serializeToString(svg);
            const svgBlob = new Blob([svgData], {type: 'image/svg+xml;charset=utf-8'});
            imageUrl = URL.createObjectURL(svgBlob);
        } catch (e) {
            console.error("SVG to data URL error:", e);
        }
    }
    
    if (imageUrl) {
        // Create a link and trigger download
        const link = document.createElement('a');
        link.href = imageUrl;
        link.download = 'qrcode.png';
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        
        // Revoke object URL if it was created from a blob
        if (imageUrl.startsWith('blob:')) {
            URL.revokeObjectURL(imageUrl);
        }
    } else {
        alert('Failed to download QR code. Please try again.');
    }
}

// Function to display fallback proxy list
function displayFallbackProxyList() {
    // Add a fallback proxy list for immediate display
    proxyList = [
        { ip: '103.6.207.108', port: '8080', country: 'ID', provider: 'PT Pusat Media Indonesia' },
        { ip: '45.8.107.73', port: '80', country: 'US', provider: 'Cloudflare Inc' },
        { ip: '172.67.181.52', port: '443', country: 'US', provider: 'Cloudflare Inc' },
        { ip: '104.21.69.85', port: '443', country: 'US', provider: 'Cloudflare Inc' },
        { ip: '185.219.132.181', port: '8080', country: 'NL', provider: 'Netherlands Provider' },
        { ip: '45.8.107.73', port: '80', country: 'UK', provider: 'British Telecom' },
        { ip: '172.67.182.77', port: '443', country: 'JP', provider: 'Japan Telecom' },
        { ip: '104.21.70.123', port: '8080', country: 'SG', provider: 'Singapore Telecom' },
        { ip: '185.219.133.45', port: '3128', country: 'DE', provider: 'Deutsche Telekom' },
        { ip: '45.8.105.26', port: '80', country: 'FR', provider: 'Orange France' },
        { ip: '172.67.183.98', port: '443', country: 'CA', provider: 'Bell Canada' },
        { ip: '104.21.71.205', port: '8080', country: 'AU', provider: 'Telstra Australia' },
        { ip: '185.219.134.67', port: '3128', country: 'IT', provider: 'Telecom Italia' },
        { ip: '45.8.104.83', port: '80', country: 'BR', provider: 'Brazil Telecom' },
        { ip: '172.67.184.12', port: '443', country: 'RU', provider: 'Russian Telecom' }
    ];
    
    filteredProxyList = [...proxyList];
    renderProxyList();
}

// Process proxy list data
function processProxyData(text) {
    // Handle different line endings and remove empty lines
    const lines = text.split(/\r?\n/).filter(line => line.trim() !== '');
    console.log(`Found ${lines.length} lines in proxy data`);
    
    if (lines.length === 0) {
        noProxiesMessage.classList.remove('hidden');
        return; // No data to process
    }
    
    // Try to determine the format of the data
    let delimiter = ','; // Default delimiter
    
    // Check if the data uses tabs or other delimiters
    const firstLine = lines[0];
    if (firstLine.includes('\t')) {
        delimiter = '\t';
    } else if (firstLine.includes('|')) {
        delimiter = '|';
    } else if (firstLine.includes(';')) {
        delimiter = ';';
    }
    
    // Parse proxy list with the detected delimiter
    proxyList = lines.map(line => {
        const parts = line.split(delimiter);
        
        // Require at least IP and port
        if (parts.length >= 2) {
            return {
                ip: parts[0].trim(),
                port: parts[1].trim(),
                country: parts.length >= 3 ? parts[2].trim() : 'Unknown',
                provider: parts.length >= 4 ? parts[3].trim() : 'Unknown Provider'
            };
        }
        return null;
    }).filter(proxy => proxy && proxy.ip && proxy.port);
    
    console.log(`Processed ${proxyList.length} valid proxies`);
    
    // If no valid proxies were found, show message and use fallback
    if (proxyList.length === 0) {
        noProxiesMessage.classList.remove('hidden');
        displayFallbackProxyList();
        return;
    }
    
    // Reset pagination
    currentPage = 1;
    filteredProxyList = [...proxyList];
    
    // Render the proxy list
    renderProxyList();
}

// Function to render the proxy list with pagination
function renderProxyList() {
    proxyListContainer.innerHTML = '';
    
    if (filteredProxyList.length === 0) {
        noProxiesMessage.classList.remove('hidden');
        paginationContainer.innerHTML = '';
        proxyCountInfo.textContent = '';
        return;
    }
    
    noProxiesMessage.classList.add('hidden');
    
    // Calculate pagination
    const totalPages = Math.ceil(filteredProxyList.length / itemsPerPage);
    const startIndex = (currentPage - 1) * itemsPerPage;
    const endIndex = Math.min(startIndex + itemsPerPage, filteredProxyList.length);
    
    // Get current page items
    const currentItems = filteredProxyList.slice(startIndex, endIndex);
    
    // Render proxy cards
    currentItems.forEach((proxy, index) => {
        const actualIndex = startIndex + index;
        const card = document.createElement('div');
        card.className = 'proxy-card group';
        
        // Create the main content of the card with forced row layout
        const cardContent = document.createElement('div');
        cardContent.className = 'flex justify-between items-center';
        cardContent.style.display = 'flex'; // Force flex display
        cardContent.style.flexDirection = 'row'; // Force row direction

        // Left side with proxy info
        const infoDiv = document.createElement('div');
        infoDiv.className = 'flex-1 min-w-0 pr-2'; // min-w-0 helps with text truncation

        // Provider and status badge container
        const providerContainer = document.createElement('div');
        providerContainer.className = 'flex-items-center';
        providerContainer.style.display = 'flex';
        providerContainer.style.alignItems = 'center';
        providerContainer.style.width = '100%';
        providerContainer.style.position = 'relative';

        // Provider name with truncation
        const providerName = document.createElement('div');
        providerName.className = 'font-medium text-sm truncate group-hover:text-indigo-300 transition-colors';
        providerName.style.maxWidth = 'calc(100% - 20px)'; // Leave space for the status indicator
        providerName.textContent = proxy.provider;
        providerContainer.appendChild(providerName);

        // Status badge (initially loading)
        const statusBadge = document.createElement('span');
        statusBadge.className = 'inline-block w-3 h-3 rounded-full bg-gray-500 ml-2 pulse-animation';
        statusBadge.style.flexShrink = '0';
        statusBadge.style.position = 'relative';
        statusBadge.innerHTML = '';
        statusBadge.title = 'Memeriksa...';
        statusBadge.id = `proxy-status-${actualIndex}`;
        providerContainer.appendChild(statusBadge);

        infoDiv.appendChild(providerContainer);

        // Country and IP:Port info with truncation
        const detailsDiv = document.createElement('div');
        detailsDiv.className = 'text-xs text-gray-400 mt-1 truncate group-hover:text-gray-300 transition-colors';
        detailsDiv.style.whiteSpace = 'nowrap';
        detailsDiv.style.overflow = 'hidden';
        detailsDiv.style.textOverflow = 'ellipsis';
        detailsDiv.textContent = `${proxy.country} | ${proxy.ip}:${proxy.port}`;
        infoDiv.appendChild(detailsDiv);

        // Right side with button - fixed width to prevent wrapping
        const buttonDiv = document.createElement('div');
        buttonDiv.className = 'flex-shrink-0';
        buttonDiv.style.flexShrink = '0'; // Prevent shrinking

        const button = document.createElement('button');
        button.className = 'create-account-btn primary-btn py-2 px-4 rounded-lg text-xs group-hover:scale-105 transition-transform';
        button.style.whiteSpace = 'nowrap';
        button.style.minWidth = '60px';
        button.setAttribute('data-index', actualIndex);
        button.innerHTML = 'Create';
        buttonDiv.appendChild(button);
        
        // Assemble the card
        cardContent.appendChild(infoDiv);
        cardContent.appendChild(buttonDiv);
        card.appendChild(cardContent);
        
        proxyListContainer.appendChild(card);
        
        // Check proxy status for this card
        checkProxyStatusInList(proxy, statusBadge);
    });
    
    // Add event listeners to create account buttons
    document.querySelectorAll('.create-account-btn').forEach(button => {
        button.addEventListener('click', function() {
            const index = parseInt(this.getAttribute('data-index'));
            selectProxy(index);
            showAccountCreationSection();
        });
    });
    
    // Render pagination controls
    renderPagination(totalPages);
    
    // Update proxy count info
    proxyCountInfo.textContent = `Showing ${startIndex + 1}-${endIndex} of ${filteredProxyList.length} proxies`;
}

// Function to check proxy status in the list
function checkProxyStatusInList(proxy, statusBadge) {
    const statusURL = `https://apicek.t-me-inconigto.workers.dev/proxy?ip=${proxy.ip}&port=${proxy.port}`;
    
    fetch(statusURL)
        .then(response => response.json())
        .then(data => {
            if (data.proxyip === true) {
                statusBadge.className = 'inline-block w-3 h-3 rounded-full bg-emerald-500 ml-2';
                statusBadge.innerHTML = '';
                statusBadge.title = 'Aktif';
            } else {
                statusBadge.className = 'inline-block w-3 h-3 rounded-full bg-rose-500 ml-2';
                statusBadge.innerHTML = '';
                statusBadge.title = 'Mati';
            }
        })
        .catch(error => {
            statusBadge.className = 'inline-block w-3 h-3 rounded-full bg-amber-500 ml-2';
            statusBadge.innerHTML = '';
            statusBadge.title = 'Tidak diketahui';
            console.error("Fetch error:", error);
        });
}

// Function to render pagination controls
function renderPagination(totalPages) {
    paginationContainer.innerHTML = '';
    
    if (totalPages <= 1) return;
    
    // Previous button
    const prevBtn = document.createElement('button');
    prevBtn.className = `pagination-btn ${currentPage === 1 ? 'disabled' : ''}`;
    prevBtn.innerHTML = '<i class="fas fa-chevron-left"></i>';
    prevBtn.disabled = currentPage === 1;
    prevBtn.addEventListener('click', () => {
        if (currentPage > 1) {
            currentPage--;
            renderProxyList();
        }
    });
    paginationContainer.appendChild(prevBtn);
    
    // Page numbers
    const maxVisiblePages = window.innerWidth < 640 ? 3 : 5;
    let startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
    let endPage = Math.min(totalPages, startPage + maxVisiblePages - 1);
    
    // Adjust start page if we're near the end
    if (endPage - startPage + 1 < maxVisiblePages) {
        startPage = Math.max(1, endPage - maxVisiblePages + 1);
    }
    
    // First page button if not visible
    if (startPage > 1) {
        const firstPageBtn = document.createElement('button');
        firstPageBtn.className = 'pagination-btn';
        firstPageBtn.textContent = '1';
        firstPageBtn.addEventListener('click', () => {
            currentPage = 1;
            renderProxyList();
        });
        paginationContainer.appendChild(firstPageBtn);
        
        // Ellipsis if needed
        if (startPage > 2) {
            const ellipsis = document.createElement('span');
            ellipsis.className = 'px-1 text-gray-400';
            ellipsis.textContent = '...';
            paginationContainer.appendChild(ellipsis);
        }
    }
    
    // Page buttons
    for (let i = startPage; i <= endPage; i++) {
        const pageBtn = document.createElement('button');
        pageBtn.className = `pagination-btn ${i === currentPage ? 'active' : ''}`;
        pageBtn.textContent = i.toString();
        pageBtn.addEventListener('click', () => {
            currentPage = i;
            renderProxyList();
        });
        paginationContainer.appendChild(pageBtn);
    }
    
    // Last page button if not visible
    if (endPage < totalPages) {
        // Ellipsis if needed
        if (endPage < totalPages - 1) {
            const ellipsis = document.createElement('span');
            ellipsis.className = 'px-1 text-gray-400';
            ellipsis.textContent = '...';
            paginationContainer.appendChild(ellipsis);
        }
        
        const lastPageBtn = document.createElement('button');
        lastPageBtn.className = 'pagination-btn';
        lastPageBtn.textContent = totalPages.toString();
        lastPageBtn.addEventListener('click', () => {
            currentPage = totalPages;
            renderProxyList();
        });
        paginationContainer.appendChild(lastPageBtn);
    }
    
    // Next button
    const nextBtn = document.createElement('button');
    nextBtn.className = `pagination-btn ${currentPage === totalPages ? 'disabled' : ''}`;
    nextBtn.innerHTML = '<i class="fas fa-chevron-right"></i>';
    nextBtn.disabled = currentPage === totalPages;
    nextBtn.addEventListener('click', () => {
        if (currentPage < totalPages) {
            currentPage++;
            renderProxyList();
        }
    });
    paginationContainer.appendChild(nextBtn);
}

// Function to select a proxy
async function selectProxy(index) {
    selectedProxy = filteredProxyList[index];
    
    // Update selected proxy info
    document.getElementById('selected-ip').textContent = selectedProxy.ip;
    document.getElementById('selected-port').textContent = selectedProxy.port;
    document.getElementById('selected-country').textContent = selectedProxy.country;
    document.getElementById('selected-provider').textContent = selectedProxy.provider;
    
    // Update form fields
    const baseAccountName = `${selectedProxy.country} - ${selectedProxy.provider}`;
    const path = `/Inconigto-Mode/${selectedProxy.ip}-${selectedProxy.port}`;

    // Set the path values
    document.getElementById('vmess-path').value = path;
    document.getElementById('vless-path').value = path;
    document.getElementById('trojan-path').value = path;
    document.getElementById('ss-path').value = path;

    // Set initial account names with protocol and TLS info
    const vmessSecurity = document.getElementById('vmess-security').value;
    const vlessSecurity = document.getElementById('vless-security').value;
    const trojanSecurity = document.getElementById('trojan-security').value;
    const ssSecurity = document.getElementById('ss-security').value;

    document.getElementById('vmess-name').value = `${baseAccountName} [VMess-${vmessSecurity === 'tls' ? 'TLS' : 'NTLS'}]`;
    document.getElementById('vless-name').value = `${baseAccountName} [VLESS-${vlessSecurity === 'tls' ? 'TLS' : 'NTLS'}]`;
    document.getElementById('trojan-name').value = `${baseAccountName} [Trojan-${trojanSecurity === 'tls' ? 'TLS' : 'NTLS'}]`;
    document.getElementById('ss-name').value = `${baseAccountName} [SS-${ssSecurity === 'tls' ? 'TLS' : 'NTLS'}]`;

    // Add event listeners to update account names when security option changes
    const securitySelects = [
        { id: 'vmess-security', nameId: 'vmess-name', protocol: 'VMess' },
        { id: 'vless-security', nameId: 'vless-name', protocol: 'VLESS' },
        { id: 'trojan-security', nameId: 'trojan-name', protocol: 'Trojan' },
        { id: 'ss-security', nameId: 'ss-name', protocol: 'SS' }
    ];

    securitySelects.forEach(item => {
        const select = document.getElementById(item.id);
        const nameInput = document.getElementById(item.nameId);
        
        // Remove any existing event listeners (to prevent duplicates)
        const newSelect = select.cloneNode(true);
        select.parentNode.replaceChild(newSelect, select);
        
        // Add new event listener
        newSelect.addEventListener('change', function() {
            const tlsType = this.value === 'tls' ? 'TLS' : 'NTLS';
            nameInput.value = `${baseAccountName} [${item.protocol}-${tlsType}]`;
        });
    });
    
    // Check proxy status in the account creation section
    const statusContainer = document.getElementById('proxy-status-container');
    const statusLoading = document.getElementById('proxy-status-loading');
    const statusActive = document.getElementById('proxy-status-active');
    const statusDead = document.getElementById('proxy-status-dead');
    const statusUnknown = document.getElementById('proxy-status-unknown');
    const latencyElement = document.getElementById('proxy-latency');
    
    // Show status container and loading state
    statusContainer.classList.remove('hidden');
    statusLoading.classList.remove('hidden');
    statusActive.classList.add('hidden');
    statusDead.classList.add('hidden');
    statusUnknown.classList.add('hidden');
    
    checkProxyStatus(selectedProxy);
}

// Function to check proxy status in the account creation section
function checkProxyStatus(proxy) {
    const startTime = performance.now();
    const statusURL = `https://apicek.t-me-inconigto.workers.dev/proxy?ip=${proxy.ip}&port=${proxy.port}`;
    const statusContainer = document.getElementById('proxy-status-container');
    const statusLoading = document.getElementById('proxy-status-loading');
    const statusActive = document.getElementById('proxy-status-active');
    const statusDead = document.getElementById('proxy-status-dead');
    const statusUnknown = document.getElementById('proxy-status-unknown');
    const latencyElement = document.getElementById('proxy-latency');
    
    // Show status container and loading state
    statusContainer.classList.remove('hidden');
    statusLoading.classList.remove('hidden');
    statusActive.classList.add('hidden');
    statusDead.classList.add('hidden');
    statusUnknown.classList.add('hidden');
    
    fetch(statusURL)
        .then(response => response.json())
        .then(data => {
            const endTime = performance.now();
            let latency = Math.floor((endTime - startTime));
            
            // Hide loading state
            statusLoading.classList.add('hidden');
            
            if (data.proxyip === true) {
                statusActive.classList.remove('hidden');
                latencyElement.textContent = `${latency}ms`;
            } else {
                statusDead.classList.remove('hidden');
            }
        })
        .catch(error => {
            // Hide loading state
            statusLoading.classList.add('hidden');
            statusUnknown.classList.remove('hidden');
            console.error("Fetch error:", error);
        });
}

// Function to show proxy list section
function showProxyListSection() {
    proxyListSection.classList.remove('hidden');
    accountCreationSection.classList.add('hidden');
    resultSection.classList.add('hidden');
}

// Function to show account creation section
function showAccountCreationSection() {
    proxyListSection.classList.add('hidden');
    accountCreationSection.classList.remove('hidden');
    resultSection.classList.add('hidden');
}

// Helper functions
function generateUUID(elementId) {
    document.getElementById(elementId).value = defaultUUID;
}

function generatePassword(elementId) {
    // Set password to the default UUID instead of generating a random one
    document.getElementById(elementId).value = defaultUUID;
}

// Update the loadProxyList function to better handle GitHub data and CORS issues
function loadProxyList(url) {
    // Show loading indicator
    loadingIndicator.classList.remove('hidden');
    proxyListContainer.innerHTML = '';
    noProxiesMessage.classList.add('hidden');
    
    // Try multiple CORS proxies in sequence
    const corsProxies = [
        // Direct fetch (no proxy)
        async () => {
            const response = await fetch(url);
            if (!response.ok) throw new Error('Direct fetch failed');
            return await response.text();
        },
        // CORS Anywhere proxy
        async () => {
            const corsUrl = `https://cors-anywhere.herokuapp.com/${url}`;
            const response = await fetch(corsUrl);
            if (!response.ok) throw new Error('CORS Anywhere proxy failed');
            return await response.text();
        },
        // AllOrigins proxy
        async () => {
            const corsUrl = `https://api.allorigins.win/get?url=${encodeURIComponent(url)}`;
            const response = await fetch(corsUrl);
            if (!response.ok) throw new Error('AllOrigins proxy failed');
            const data = await response.json();
            return data.contents;
        },
        // CORS.sh proxy
        async () => {
            const corsUrl = `https://cors.sh/${url}`;
            const response = await fetch(corsUrl, {
                headers: {
                    'x-cors-api-key': 'temp_' + Math.random().toString(36).substring(2,12),
                }
            });
            if (!response.ok) throw new Error('CORS.sh proxy failed');
            return await response.text();
        }
    ];
    
    // Try each proxy in sequence
    (async function tryProxies(index = 0) {
        if (index >= corsProxies.length) {
            console.error('All proxies failed');
            loadingIndicator.classList.add('hidden');
            noProxiesMessage.classList.remove('hidden');
            // Fall back to sample data
            displayFallbackProxyList();
            return;
        }
        
        try {
            const text = await corsProxies[index]();
            console.log("Fetched data:", text.substring(0, 200) + "..."); // Debug log (truncated)
            processProxyData(text);
            loadingIndicator.classList.add('hidden');
        } catch (error) {
            console.error(`Proxy ${index} failed:`, error);
            // Try next proxy
            tryProxies(index + 1);
        }
    })();
}
</script>
</body>
</html>
"#;

    // Return HTML response
    Response::from_html(html)
}
