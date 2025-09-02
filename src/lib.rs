mod common;
mod config;
mod proxy;

use crate::config::Config;
use crate::proxy::*;

use std::collections::HashMap;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use serde_json::json;
use uuid::Uuid;
use worker::*;
use once_cell::sync::Lazy;
use regex::Regex;

static PROXYIP_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^.+-\d+$").unwrap());
static PROXYKV_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([A-Z]{2})").unwrap());

// Base URL for GitHub raw content
static GITHUB_BASE_URL: &str = "https://raw.githubusercontent.com/ujangkasep2504/Siren-kasep/refs/heads/master/web";

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let uuid = env
        .var("UUID")
        .map(|x| Uuid::parse_str(&x.to_string()).unwrap_or_default())?;
    let host = req.url()?.host().map(|x| x.to_string()).unwrap_or_default();
    let main_page_url = env.var("MAIN_PAGE_URL").map(|x| x.to_string()).unwrap();
    let sub_page_url = env.var("SUB_PAGE_URL").map(|x| x.to_string()).unwrap();
    let link_page_url = env.var("LINK_PAGE_URL").map(|x| x.to_string()).unwrap();
    let converter_page_url = env.var("CONVERTER_PAGE_URL").map(|x| x.to_string()).unwrap();

    let config = Config { 
        uuid, 
        host: host.clone(), 
        proxy_addr: host, 
        proxy_port: 443, 
        main_page_url, 
        sub_page_url,
        link_page_url,
        converter_page_url,
    };

    // Get the URL path to determine if it's a resource request
    let url = req.url()?;
    let path = url.path();

    // Check if the request is for a CSS, JS, or image file
    if path.starts_with("/css/") {
        return handle_css_file(req).await;
    } else if path.starts_with("/js/") {
        return handle_js_file(req).await;
    } else if path.starts_with("/images/") {
        return handle_image_file(req).await;
    }

    // For other routes, use the router - EXACTLY as in the original
    Router::with_data(config)
        .on_async("/", fe)
        .on_async("/sub", sub)
        .on_async("/link", link)
        .on_async("/converter", converter)
        .on_async("/:proxyip", tunnel)
        .on_async("/Inconigto-Mode/:proxyip", tunnel)
        .run(req, env)
        .await
}

// Handler for CSS files - fetch from GitHub
async fn handle_css_file(req: Request) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    
    // Extract the CSS filename from the path
    let filename = path.strip_prefix("/css/").unwrap_or("");
    
    // Construct the GitHub URL for the CSS file
    let css_url = format!("{}/css/{}", GITHUB_BASE_URL, filename);
    
    // Fetch the CSS file from GitHub
    let req = Fetch::Url(Url::parse(&css_url)?);
    let mut res = req.send().await?;
    
    if res.status_code() == 200 {
        let css = res.text().await?;
        
        // Create a response with the CSS content and appropriate headers
        let mut headers = Headers::new();
        headers.set("Content-Type", "text/css")?;
        
        // Add caching headers
        headers.set("Cache-Control", "public, max-age=86400")?; // Cache for 1 day
        
        Ok(Response::ok(css)?.with_headers(headers))
    } else {
        Response::error("CSS file not found", 404)
    }
}

// Handler for JS files - fetch from GitHub
async fn handle_js_file(req: Request) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    
    // Extract the JS filename from the path
    let filename = path.strip_prefix("/js/").unwrap_or("");
    
    // Construct the GitHub URL for the JS file
    let js_url = format!("{}/js/{}", GITHUB_BASE_URL, filename);
    
    // Fetch the JS file from GitHub
    let req = Fetch::Url(Url::parse(&js_url)?);
    let mut res = req.send().await?;
    
    if res.status_code() == 200 {
        let js = res.text().await?;
        
        // Create a response with the JS content and appropriate headers
        let mut headers = Headers::new();
        headers.set("Content-Type", "application/javascript")?;
        
        // Add caching headers
        headers.set("Cache-Control", "public, max-age=86400")?; // Cache for 1 day
        
        Ok(Response::ok(js)?.with_headers(headers))
    } else {
        Response::error("JavaScript file not found", 404)
    }
}

// Handler for image files - fetch from GitHub
async fn handle_image_file(req: Request) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    
    // Extract the image filename from the path
    let filename = path.strip_prefix("/images/").unwrap_or("");
    
    // Construct the GitHub URL for the image file
    let image_url = format!("{}/images/{}", GITHUB_BASE_URL, filename);
    
    // Fetch the image file from GitHub
    let req = Fetch::Url(Url::parse(&image_url)?);
    let mut res = req.send().await?;
    
    if res.status_code() == 200 {
        let image_data = res.bytes().await?;
        
        // Create a response with the image content and appropriate headers
        let mut headers = Headers::new();
        
        // Set content type based on file extension
        if filename.ends_with(".png") {
            headers.set("Content-Type", "image/png")?;
        } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            headers.set("Content-Type", "image/jpeg")?;
        } else if filename.ends_with(".svg") {
            headers.set("Content-Type", "image/svg+xml")?;
        } else if filename.ends_with(".gif") {
            headers.set("Content-Type", "image/gif")?;
        } else {
            headers.set("Content-Type", "application/octet-stream")?;
        }
        
        // Add caching headers
        headers.set("Cache-Control", "public, max-age=86400")?; // Cache for 1 day
        
        Ok(Response::from_bytes(image_data)?.with_headers(headers))
    } else {
        Response::error("Image file not found", 404)
    }
}

// KEEP ALL ORIGINAL FUNCTIONS EXACTLY AS THEY WERE

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

async fn link(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.link_page_url).await
}

async fn converter(_: Request, cx: RouteContext<Config>) -> Result<Response> {
    get_response_from_url(cx.data.converter_page_url).await
}

// KEEP THE TUNNEL FUNCTION EXACTLY AS IT WAS IN THE ORIGINAL
async fn tunnel(req: Request, mut cx: RouteContext<Config>) -> Result<Response> {
    let mut proxyip = cx.param("proxyip").unwrap().to_string();
    if PROXYKV_PATTERN.is_match(&proxyip)  {
        let kvid_list: Vec<String> = proxyip.split(",").map(|s| s.to_string()).collect();
        let kv = cx.kv("SIREN")?;
        let mut proxy_kv_str = kv.get("proxy_kv").text().await?.unwrap_or("".to_string());
        let mut rand_buf = [0u8, 1];
        getrandom::getrandom(&mut rand_buf).expect("failed generating random number");

        if proxy_kv_str.len() == 0 {
            console_log!("getting proxy kv from github...");
            let req = Fetch::Url(Url::parse("https://raw.githubusercontent.com/FoolVPN-ID/Nautica/refs/heads/main/kvProxyList.json")?);
            let mut res = req.send().await?;
            if res.status_code() == 200 {
                proxy_kv_str = res.text().await?.to_string();
                kv.put("proxy_kv", &proxy_kv_str)?.expiration_ttl(60 * 60 * 24).execute().await?; // 24 hours
            } else {
                return Err(Error::from(format!("error getting proxy kv: {}", res.status_code())));
            }
        }

        let proxy_kv: HashMap<String, Vec<String>> = serde_json::from_str(&proxy_kv_str)?;

        let kv_index = (rand_buf[0] as usize) % kvid_list.len();
        proxyip = kvid_list[kv_index].clone();

        let proxyip_index = (rand_buf[0] as usize) % proxy_kv[&proxyip].len();
        proxyip = proxy_kv[&proxyip][proxyip_index].clone().replace(":", "-");
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
