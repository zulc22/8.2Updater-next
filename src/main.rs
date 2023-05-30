use chrono::naive::NaiveDateTime;
use directories::ProjectDirs;
use regex::Regex;
use reqwest::blocking;
use scraper::{Html, Selector};
use trauma::downloader::DownloaderBuilder;

use std::{
    fs,
    io::{self, Read, Write},
    panic::PanicInfo,
    path::{self, Path},
    process::Command,
};

extern crate chrono;
extern crate regex;
extern crate reqwest;
extern crate scraper;
extern crate sevenz_rust;
extern crate tokio;
extern crate trauma;

// from https://users.rust-lang.org/t/rusts-equivalent-of-cs-system-pause/4494/4
fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    stdout.write(b"Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn panicfunc(info: &PanicInfo) {
    println!("Failed to update: {}.", info.to_string());
    println!("Overriding and starting GM8.2 anyway.");
    startgm82();
    println!("If this error seems like it shouldn't have happened, talk to zulc22 about it");
    pause();
}

fn get_body(url: &str) -> String {
    let body: String = blocking::get(url)
        .expect("unable to read from online server")
        .error_for_status()
        .expect("server did not return 200 (OK)")
        .text()
        .expect("could not decode response body");
    return body;
}

async fn updategm82_as(url: &str, p: &Path, update: &str) {
    println!("\nDownloading GM8.2...");
    let mut d = trauma::download::Download::try_from(url).unwrap();
    d.filename = "gm82.7z".to_string();
    DownloaderBuilder::new()
        .directory(p.to_path_buf())
        .build()
        .download(&[d])
        .await;
    println!("Extracting...");
    sevenz_rust::decompress_file(p.join("gm82.7z"), p.join("gm82")).expect("unable to extract");
    println!("Installing...");
    Command::new("cmd.exe")
        .args([
            "/c",
            "cd",
            p.join("gm82").to_str().unwrap(),
            "&",
            "install.bat",
        ])
        .spawn()
        .expect("failed to run installer")
        .wait()
        .unwrap();
    fs::remove_dir_all(p.join("gm82")).unwrap();
    fs::write(p.join("lastupdated.txt"), update).expect("couldn't write update date");
}

fn updategm82(url: &str, p: &Path, update: &str) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(updategm82_as(url, p, update));
}

fn startgm82() {
    println!("\nStarting GM8.2...");
    Command::new("cmd.exe")
        .args(["/c", "start", "", r#"%appdata%\GameMaker8.2\GameMaker.exe"#])
        .spawn()
        .unwrap();
}

fn main() {
    std::panic::set_hook(Box::new(|info| panicfunc(info)));
    println!("8.2Updater-next, rev 1\nzulc22 2023");

    let configdir: ProjectDirs =
        ProjectDirs::from("com", "zulc22", "8.2Updater-next").expect("couldn't get projectdir");
    let d = configdir.config_dir();

    if !d.is_dir() {
        let mut msg = "can't create appdata dir ".to_owned();
        msg.push_str(d.to_str().unwrap());
        fs::create_dir_all(d).expect(&msg);
    }

    print!("\nYour copy of GM8.2 was last updated: ");
    let last_update_file: path::PathBuf = d.join("lastupdated.txt");
    let mut last_update: String = "".to_string();
    if last_update_file.exists() {
        let mut f = fs::File::open(last_update_file).expect("can't open file");
        let mut data = vec![];
        f.read_to_end(&mut data).expect("can't read file");
        last_update = String::from_utf8(data).expect("can't decode file");
        println!("{last_update}")
    } else {
        println!("N/A (unknown)");
    }

    println!("\nConnecting to Mediafire...");

    let body = get_body("https://www.mediafire.com/file/qe4d1zv9p3gp4h7");

    println!("Parsing response...");
    let body_parsed = Html::parse_document(&body);

    let element = body_parsed
        .select(&Selector::parse(".details li").unwrap())
        .nth(1)
        .expect("couldn't extract date")
        .select(&Selector::parse("span").unwrap())
        .next()
        .unwrap();

    let datestr = element.text().next().unwrap();

    println!("GM8.2 was last updated on Mediafire: {datestr}");

    let re = Regex::new(r"https:\/\/download.*?\.mediafire\.com\/.*?\.7z").unwrap();
    let downloadlink = re
        .captures_iter(&body)
        .next()
        .expect("couldn't find download link")
        .get(0)
        .unwrap()
        .as_str();

    if last_update == "" {
        updategm82(downloadlink, Path::new(configdir.config_dir()), datestr);
    } else if last_update != datestr {
        let remotedate =
            NaiveDateTime::parse_from_str(datestr, "%Y-%m-%d %H:%M:%S").expect("can't parse time");
        let localdate = NaiveDateTime::parse_from_str(&last_update, "%Y-%m-%d %H:%M:%S")
            .expect("can't parse localtime");

        if localdate < remotedate {
            updategm82(downloadlink, Path::new(configdir.config_dir()), datestr);
        }
    } else if last_update == datestr {
        println!("GM8.2 is up to date!");
        startgm82();
    } else {
        panic!("i don't know how this edge case could fire but it's probably not good")
    }
}
