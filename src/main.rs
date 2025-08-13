use clap::{Arg, Command};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::process;

fn create_progress_bar(quiet_mode: bool, msg: &str, length: Option<u64>) -> ProgressBar {
    let bar = if quiet_mode {
        ProgressBar::hidden()
    } else {
        match length {
            Some(len) => ProgressBar::new(len),
            None => ProgressBar::new_spinner(),
        }
    };

    bar.set_message(msg.to_string());

    if let Some(_) = length {
        let style = ProgressStyle::default_bar()
            .template("{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}")
            .unwrap()
            .progress_chars("=> ");
        bar.set_style(style);
    } else {
        bar.set_style(ProgressStyle::default_spinner());
    }

    bar
}

fn save_to_file(data: &[u8], filename: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(filename)?;
    file.write_all(data)?;
    Ok(())
}

fn download(target: &str, quiet_mode: bool) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut resp = client.get(target).send()?;

    println!(
        "HTTP request sent... {}",
        style(format!("{}", resp.status())).green()
    );

    if resp.status().is_success() {
        let headers = resp.headers().clone();
        let ct_len = headers
            .get(CONTENT_LENGTH)
            .and_then(|val| val.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let ct_type = headers
            .get(CONTENT_TYPE)
            .and_then(|val| val.to_str().ok())
            .unwrap_or("unknown");

        match ct_len {
            Some(len) => {
                println!(
                    "Length: {} ({})",
                    style(len).green(),
                    style(format!("{}", HumanBytes(len))).red()
                );
            }
            None => {
                println!("Length: {}", style("unknown").red());
            }
        }

        println!("Type: {}", style(ct_type).green());

        let fname = target
            .split('/')
            .last()
            .unwrap_or("downloaded.file");

        println!("Saving to: {}", style(fname).green());

        let chunk_size = match ct_len {
            Some(x) => (x as usize / 99).max(1),
            None => 1024usize,
        };

        let mut buf = Vec::new();
        let bar = create_progress_bar(quiet_mode, fname, ct_len);

        loop {
            let mut buffer = vec![0; chunk_size];
            let bcount = resp.read(&mut buffer)?;
            if bcount == 0 {
                break;
            }
            buffer.truncate(bcount);
            buf.extend_from_slice(&buffer);
            bar.inc(bcount as u64);
        }

        bar.finish_with_message("Download complete");
        save_to_file(&buf, fname)?;
    }

    Ok(())
}

/// Human-readable byte formatter
struct HumanBytes(u64);
impl std::fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let mut size = self.0 as f64;
        let mut unit = 0;
        while size >= 1024.0 && unit < UNITS.len() - 1 {
            size /= 1024.0;
            unit += 1;
        }
        write!(f, "{:.2} {}", size, UNITS[unit])
    }
}

fn main() {
    let matches = Command::new("Rget")
        .version("0.1.0")
        .author("Your MOM")
        .about("wget clone written in Rust")
        .arg(
            Arg::new("URL")
                .required(true)
                .index(1)
                .help("URL to download"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Disable progress bar"),
        )
        .get_matches();

    let url = matches.get_one::<String>("URL").unwrap();
    let quiet_mode = matches.contains_id("quiet");

    if let Err(e) = download(url, quiet_mode) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}