use logwatcher;

// use std::io::Read;
use curl::easy::{Easy, List};
use std::path::Path;
use std::{thread, time};
use chrono::{DateTime, Utc};

pub fn load(file_name:String, load_url:String) {

    let f = file_name.clone();
    let file_path = Path::new(&f);
    let mut tries = 0;

    while !file_path.exists() {
        tries += 1;
        println!("No log file - {}", file_name);
        thread::sleep(time::Duration::from_millis(tries * 1000));

        if tries > 50 {
            panic!("No log file {}", file_name);
        }
    }

    println!("Send logs to elastic: {}", load_url);
    let mut log_watcher = logwatcher::LogWatcher::register(file_name).unwrap();

    log_watcher.watch(&mut |line: String| {

        let mut easy = Easy::new();
        let now: DateTime<Utc> = Utc::now();
        let url_with_date = format!("{}-{}/logs", load_url,  now.format("%Y-%m-%d"));
        easy.url(&url_with_date).unwrap();
        let mut list = List::new();
        list.append("Content-Type:application/json").unwrap();
        easy.post(true).unwrap();
        easy.http_headers(list).unwrap();
        let pbody = easy.post_fields_copy(line.as_bytes());
        match pbody {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err),
        }

        let res = easy.perform();

        match res {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err),
        }

        if let Ok(code) = easy.response_code() {
            if code > 299 {
                println!("St: {}", code);
            }
        }

    });
    println!("Done");
}
