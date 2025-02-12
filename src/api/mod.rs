/// Man... told you this was a mess
use curl::easy::Easy2;
use std::env;
use std::fs::File;
use std::sync::{Arc, Mutex};
pub mod chunks;
pub mod handlers;
use crate::errors::SwishError;
use curl::easy::List;
use handlers::DataHandler;
use handlers::DownloadHandler;
use handlers::UploadHandler;
use indicatif::{ProgressBar, ProgressStyle};
use crate::ca_bundle::CERT_BUNDLE;

use log;

const DEFAULT_HEADERS: &[&str; 3] = &[
    "User-Agent: swisstransfer-webext/1.0",
    "Cookie: webext=1",
    "Referer: swish/1.0.1",
];


fn new_easy2_data(
    url: String,
    custom_headers: Option<Vec<String>>,
    post: bool,
) -> Result<Easy2<DataHandler>, curl::Error> {
    let mut easy2 = Easy2::new(DataHandler { data: Vec::new() });
    if env::var("CURL_VERBOSE") == Ok("1".to_string()) {
        let _ = easy2.verbose(true);
    }

    if env::var("CURL_INSECURE") == Ok("1".to_string()) {
        let _ = easy2.ssl_verify_host(false);
        let _ = easy2.ssl_verify_peer(false);
    } else {
        if env::var("CURL_USE_INTERNAL_CA_BUNDLE") == Ok("1".to_string()) {
            if easy2.ssl_cainfo_blob(CERT_BUNDLE) != Ok(()) {
                log::error!("Failed to load cacert.pem");
            }
        }
    }

    let mut merged_headers: Vec<String> = DEFAULT_HEADERS.iter().map(|x| x.to_string()).collect();

    if post {
        easy2.post(true)?;

        //add headers
        merged_headers.push("Content-Type: application/json".to_string());
        merged_headers.push("Accept: application/json".to_string());
    }

    //add additional headers if any
    if let Some(custom_headers) = custom_headers {
        for header in custom_headers {
            merged_headers.push(header);
        }
    }

    let mut headers: List = List::new();
    for header in merged_headers {
        headers.append(header.as_str())?;
    }

    easy2.url(&url)?;
    easy2.http_headers(headers)?;

    Ok(easy2)
}

pub fn new_easy2_download(
    url: String,
    custom_headers: Option<Vec<String>>,
    file: File,
    file_size: u64,
) -> Result<Easy2<DownloadHandler<File>>, curl::Error> {
    let progress_bar = ProgressBar::new(file_size as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
    .progress_chars("#>-"));

    let mut easy2 = Easy2::new(DownloadHandler {
        writer: file.try_clone().unwrap(), // Clone the file for the handler
        progress: Arc::new(Mutex::new(progress_bar)),
    });

    if env::var("CURL_VERBOSE") == Ok("1".to_string()) {
        let _ = easy2.verbose(true);
    }

    if env::var("CURL_USE_INTERNAL_CA_BUNDLE") == Ok("1".to_string()) {
        if easy2.ssl_cainfo_blob(CERT_BUNDLE) != Ok(()) {
            log::error!("Failed to load cacert.pem");
        }
    }
    
    let mut merged_headers: Vec<String> = DEFAULT_HEADERS.iter().map(|x| x.to_string()).collect();

    //add additional headers if any
    if let Some(custom_headers) = custom_headers {
        for header in custom_headers {
            merged_headers.push(header);
        }
    }

    let mut headers: List = List::new();
    for header in merged_headers {
        headers.append(header.as_str())?;
    }

    easy2.url(&url)?;
    easy2.http_headers(headers)?;

    Ok(easy2)
}

pub fn new_easy2_upload(
    url: String,
    custom_headers: Option<Vec<String>>,
    upload_total_size: usize,
    reader: &File,
) -> Result<Easy2<UploadHandler<&File>>, curl::Error> {
    let progress_bar = ProgressBar::new(upload_total_size as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
        .progress_chars("#>-"));

    let mut easy2 = Easy2::new(UploadHandler {
        reader,
        progress: Arc::new(Mutex::new(progress_bar)),
    });

    if env::var("CURL_VERBOSE") == Ok("1".to_string()) {
        let _ = easy2.verbose(true);
    }

    let mut merged_headers: Vec<String> = DEFAULT_HEADERS.iter().map(|x| x.to_string()).collect();

    // add headers
    merged_headers.push("Content-Type: application/json".to_string());
    merged_headers.push("Accept: application/json".to_string());

    // add additional headers if any
    if let Some(custom_headers) = custom_headers {
        for header in custom_headers {
            merged_headers.push(header);
        }
    }

    let mut headers: List = List::new();
    for header in merged_headers {
        headers.append(header.as_str())?;
    }

    if env::var("CURL_INSECURE") == Ok("1".to_string()) {
        let _ = easy2.ssl_verify_host(false);
        let _ = easy2.ssl_verify_peer(false);
    } else {
        if env::var("CURL_USE_INTERNAL_CA_BUNDLE") == Ok("1".to_string()) {
            if easy2.ssl_cainfo_blob(&CERT_BUNDLE) != Ok(()) {
                log::error!("Failed to load cacert.pem");
            }
        }
    }

    easy2.url(&url)?;
    easy2.http_headers(headers)?;
    easy2.post(true)?;
    easy2.upload(true)?;
    easy2.http_version(curl::easy::HttpVersion::V11)?;

    Ok(easy2)
}

pub fn get(url: &str, additional_headers: Option<Vec<String>>) -> Result<String, SwishError> {
    let additional_headers2 = additional_headers.clone();
    let mut easy2 = new_easy2_data(url.to_string(), additional_headers, false)?;

    if env::var("CURL_VERBOSE") == Ok("1".to_string()) {
        let _ = easy2.verbose(true);
    }

    if env::var("CURL_USE_INTERNAL_CA_BUNDLE") == Ok("1".to_string()) {
        if easy2.ssl_cainfo_blob(CERT_BUNDLE) != Ok(()) {
            log::error!("Failed to load cacert.pem");
        }
    }

    log::debug!(
        "Sending get request to: {} \n with headers {}",
        url,
        additional_headers2.unwrap_or_default().join("\n")
    );

    easy2.perform()?;

    log::debug!(
        "Response: {} - {:?}",
        easy2.response_code()?,
        String::from_utf8(easy2.get_ref().data.clone()).unwrap()
    );

    match easy2.response_code()? {
        200 => {
            let data = easy2.get_ref().data.clone();
            Ok(String::from_utf8(data).unwrap())
        }

        404 => Err(SwishError::NotFound {
            url: url.to_string(),
        }),
        code => Err(SwishError::InvalidResponse {
            response: code.to_string(),
        }),
    }
}

pub fn post(
    url: &str,
    body: Vec<u8>,
    additional_headers: Option<Vec<String>>,
) -> Result<Vec<u8>, SwishError> {
    log::debug!("Sending post request to: {}", url);
    log::debug!("Body: {}", String::from_utf8(body.clone()).unwrap());
    let mut retries = 0;

    let mut easy2 = new_easy2_data(url.to_string(), additional_headers, true)?;
    if env::var("CURL_VERBOSE") == Ok("1".to_string()) {
        let _ = easy2.verbose(true);
    }

    if env::var("CURL_USE_INTERNAL_CA_BUNDLE") == Ok("1".to_string()) {
        if easy2.ssl_cainfo_blob(CERT_BUNDLE) != Ok(()) {
            log::error!("Failed to load cacert.pem");
        }
    }

    easy2.post_fields_copy(&body)?;

    loop {
        easy2.perform()?;
        log::debug!(
            "Response: {} - {:?}",
            easy2.response_code()?,
            String::from_utf8(easy2.get_ref().data.clone()).unwrap()
        );

        if easy2.response_code()? < 400 || retries >= 3 {
            let data = easy2.get_ref().data.clone();
            return Ok(data);
        } else {
            println!("Request failed, retrying... ({})", retries);
            retries += 1;
        }
    }
}
