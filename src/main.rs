use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use uuid::Uuid;

const UPLOAD_DIR: &str = "/tmp/pdf2html";
const OUTPUT_DIR: &str = "/tmp/pdf2html/output";
const CLEANUP_INTERVAL_SECS: u64 = 600; // 10分钟检查一次
const FILE_MAX_AGE_SECS: u64 = 3 * 60 * 60; // 3小时

#[derive(Serialize)]
struct ConvertResponse {
    success: bool,
    message: String,
    html_url: Option<String>,
    filename: Option<String>,
}

#[derive(Deserialize)]
struct ConvertOptions {
    zoom: Option<f32>,
    fit_width: Option<u32>,
    fit_height: Option<u32>,
    embed_css: Option<bool>,
    embed_font: Option<bool>,
    embed_image: Option<bool>,
    embed_javascript: Option<bool>,
    split_pages: Option<bool>,
    first_page: Option<u32>,
    last_page: Option<u32>,
}

/// 清理超过3小时的文件
async fn cleanup_old_files() {
    loop {
        tokio::time::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;
        
        if let Ok(entries) = std::fs::read_dir(OUTPUT_DIR) {
            let now = SystemTime::now();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(created) = metadata.created().or_else(|_| metadata.modified()) {
                            if let Ok(age) = now.duration_since(created) {
                                if age.as_secs() > FILE_MAX_AGE_SECS {
                                    if let Err(e) = std::fs::remove_dir_all(&path) {
                                        eprintln!("Failed to remove {:?}: {}", path, e);
                                    } else {
                                        println!("Cleaned up old directory: {:?}", path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 清理上传目录中的残留文件
        if let Ok(entries) = std::fs::read_dir(UPLOAD_DIR) {
            let now = SystemTime::now();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age.as_secs() > FILE_MAX_AGE_SECS {
                                    std::fs::remove_file(&path).ok();
                                    println!("Cleaned up old file: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


async fn convert_pdf(mut payload: Multipart) -> Result<HttpResponse> {
    std::fs::create_dir_all(UPLOAD_DIR).ok();
    std::fs::create_dir_all(OUTPUT_DIR).ok();

    let task_id = Uuid::new_v4().to_string();
    let task_output_dir = format!("{}/{}", OUTPUT_DIR, task_id);
    std::fs::create_dir_all(&task_output_dir).ok();

    let mut pdf_path: Option<PathBuf> = None;
    let mut options = ConvertOptions {
        zoom: None,
        fit_width: None,
        fit_height: None,
        embed_css: Some(true),
        embed_font: Some(true),
        embed_image: Some(true),
        embed_javascript: Some(true),
        split_pages: None,
        first_page: None,
        last_page: None,
    };
    let mut original_filename = String::new();

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.and_then(|cd| cd.get_name()).unwrap_or("");

        match field_name {
            "file" => {
                if let Some(filename) = content_disposition.and_then(|cd| cd.get_filename()) {
                    original_filename = sanitize_filename::sanitize(filename);
                    let file_path = PathBuf::from(format!("{}/{}", UPLOAD_DIR, original_filename));
                    let mut file = std::fs::File::create(&file_path)
                        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
                    
                    while let Some(chunk) = field.next().await {
                        let data = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                        file.write_all(&data)
                            .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
                    }
                    pdf_path = Some(file_path);
                }
            }
            "zoom" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.zoom = s.trim().parse().ok();
                }
            }
            "fit_width" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.fit_width = s.trim().parse().ok();
                }
            }
            "fit_height" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.fit_height = s.trim().parse().ok();
                }
            }
            "embed_css" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.embed_css = Some(s.trim() == "true" || s.trim() == "1");
                }
            }
            "embed_font" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.embed_font = Some(s.trim() == "true" || s.trim() == "1");
                }
            }
            "embed_image" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.embed_image = Some(s.trim() == "true" || s.trim() == "1");
                }
            }
            "embed_javascript" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.embed_javascript = Some(s.trim() == "true" || s.trim() == "1");
                }
            }
            "split_pages" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.split_pages = Some(s.trim() == "true" || s.trim() == "1");
                }
            }
            "first_page" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.first_page = s.trim().parse().ok();
                }
            }
            "last_page" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let d = chunk.map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
                    data.extend_from_slice(&d);
                }
                if let Ok(s) = String::from_utf8(data) {
                    options.last_page = s.trim().parse().ok();
                }
            }
            _ => {
                while let Some(_) = field.next().await {}
            }
        }
    }

    let pdf_path = match pdf_path {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::BadRequest().json(ConvertResponse {
                success: false,
                message: "No PDF file uploaded".to_string(),
                html_url: None,
                filename: None,
            }));
        }
    };

    let output_filename = original_filename.replace(".pdf", ".html").replace(".PDF", ".html");

    let mut args = vec![
        "--dest-dir".to_string(),
        task_output_dir.clone(),
        pdf_path.to_string_lossy().to_string(),
    ];

    if let Some(zoom) = options.zoom {
        args.push("--zoom".to_string());
        args.push(zoom.to_string());
    }
    if let Some(fit_width) = options.fit_width {
        args.push("--fit-width".to_string());
        args.push(fit_width.to_string());
    }
    if let Some(fit_height) = options.fit_height {
        args.push("--fit-height".to_string());
        args.push(fit_height.to_string());
    }
    if options.embed_css == Some(false) {
        args.push("--embed-css".to_string());
        args.push("0".to_string());
    }
    if options.embed_font == Some(false) {
        args.push("--embed-font".to_string());
        args.push("0".to_string());
    }
    if options.embed_image == Some(false) {
        args.push("--embed-image".to_string());
        args.push("0".to_string());
    }
    if options.embed_javascript == Some(false) {
        args.push("--embed-javascript".to_string());
        args.push("0".to_string());
    }
    if options.split_pages == Some(true) {
        args.push("--split-pages".to_string());
        args.push("1".to_string());
    }
    if let Some(first_page) = options.first_page {
        args.push("--first-page".to_string());
        args.push(first_page.to_string());
    }
    if let Some(last_page) = options.last_page {
        args.push("--last-page".to_string());
        args.push(last_page.to_string());
    }

    let output = Command::new("pdf2htmlEX")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    std::fs::remove_file(&pdf_path).ok();

    match output {
        Ok(output) => {
            let output_path = format!("{}/{}", task_output_dir, output_filename);
            if std::path::Path::new(&output_path).exists() {
                Ok(HttpResponse::Ok().json(ConvertResponse {
                    success: true,
                    message: "Conversion successful".to_string(),
                    html_url: Some(format!("/output/{}/{}", task_id, output_filename)),
                    filename: Some(output_filename),
                }))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(HttpResponse::InternalServerError().json(ConvertResponse {
                    success: false,
                    message: format!("Conversion failed. stderr: {} stdout: {}", stderr, stdout),
                    html_url: None,
                    filename: None,
                }))
            }
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ConvertResponse {
            success: false,
            message: format!("Failed to execute pdf2htmlEX: {}", e),
            html_url: None,
            filename: None,
        })),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all(UPLOAD_DIR).ok();
    std::fs::create_dir_all(OUTPUT_DIR).ok();

    // 启动后台清理任务
    tokio::spawn(cleanup_old_files());

    println!("Starting server at http://0.0.0.0:8080");
    println!("Files will be automatically cleaned up after 3 hours");
    
    HttpServer::new(|| {
        App::new()
            .route("/api/convert", web::post().to(convert_pdf))
            .service(Files::new("/output", OUTPUT_DIR).show_files_listing())
            .service(Files::new("/", "/app/static").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
