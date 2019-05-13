use logwatcher;

use std::io::Read;
use curl::easy::{Easy, List};

pub fn load(file_name:String, load_url:String) {
    let mut log_watcher = logwatcher::LogWatcher::register(file_name).unwrap();

    println!("Send logs to elastic: {}", load_url);

    let mut easy = Easy::new();
    easy.url(&load_url).unwrap();
    let mut list = List::new();
    list.append("Content-Type:application/json").unwrap();
    easy.post(true).unwrap();
    easy.http_headers(list).unwrap();

    log_watcher.watch(&mut |line: String| {

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
