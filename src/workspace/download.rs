use std::{fs::File, io::copy};

use reqwest::{StatusCode, blocking::Client};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

pub fn download(name: String, url: String, filepath: String) -> Result<(), String> {
    let size = download_silent(name, url, filepath.clone())?;

    println!("Copied {:.3} MB into {filepath}", size);

    Ok(())
}

pub fn download_silent(name: String, url: String, filepath: String) -> Result<f32, String> {
    let client: Client = Client::new();
    let mut response = match client.get(&url).header("User-Agent", USER_AGENT).send() {
        Ok(r) => r,
        Err(e) => {
            return Err(format!(
                "{url}, status: {}",
                e.status()
                    .unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
                    .as_str()
            ));
        }
    };

    if !response.status().is_success() {
        return Err(format!("{url}, status: {}", response.status().as_str()));
    }

    let mut file = File::create(&filepath)
        .map_err(|e| format!("Could not create file {filepath} for dependency {name}: {e}"))?;

    match copy(&mut response, &mut file) {
        Ok(v) => Ok(v as f32 / 1000000.0),
        Err(e) => Err(format!(
            "Could not copy from URL {url} into file {filepath}: {e}"
        )),
    }
}
