// use std::env;
// use std::fs::File;
extern crate serde_json;
extern crate colored;
extern crate hyper;
extern crate curl;

extern crate clap;
extern crate base64;
extern crate postgres;
extern crate postgres_binary_copy;

use std::io;
use serde_json::{Value};
use colored::*;
use std::collections::HashMap;

use std::io::prelude::*;
// use std::io::{Write};
use hyper::Client;
use hyper::rt::{self, Future, Stream};
use clap::{Arg, App, SubCommand};

mod logwatcher;
mod pgloader;
mod esloader;
// use hyper::header::{Headers, Authorization, Basic};


fn format_ts(ts: String) -> String {

    // let millis = ts % 1000;
    // let second = (ts / 1000) % 60;
    // let minute = (ts / (1000 * 60)) % 60;
    // let hour = (ts / (1000 * 60 * 60)) % 24;

    // return format!("{:0>2}:{:0>2}:{:0>2}", hour, minute, second);
    return ts;
}

fn format_d(v:Value) -> ColoredString {
    match v["d"].as_i64() {
        Some(s) => {
            let f = format!(" [{}ms]", s);
            if s > 50 {
                return f.bright_red()
            } else {
                return f.normal()
            }
        }
        None => return "".normal()
    }
}


fn format_line(v:Value) -> String {
    let mut res = String::new();

    if v["ev"].is_string() && v["ts"].is_string() {
        let ev = match v["ev"].as_str() {
            Some(s) => s,
            None => "Unknown"
        };

        if let Some(tn) = v["tn"].as_str(){
            res.push_str(&format!("{} ", tn.bright_green()));
        }

        match v["ts"].as_str() {
            Some(ts) =>  res.push_str(&format!("{}",format_ts(String::from(ts)).bright_black())),
            None => ()
        }
        if ev == "w/req" {
            let m = match v["w_m"].as_str() {
                Some(s) => s.to_uppercase().bold().yellow(),
                None => "GET".bold().yellow()
            };

            let url = match v["w_url"].as_str() {
                Some(s) => s.white().bold(),
                None => "???".white().bold()
            };

            res.push_str(&format!(" {} {}",m, url));

            match v["w_qs"].as_str() {
                Some(qs) => res.push_str(&format!("?{}", qs)),
                None => ()
            }
            match v["w_addr"].as_str() {
                Some(s) => res.push_str(&format!(" by {}", s.bright_black())),
                None => ()
            }
        } else if ev == "db/q" || ev == "db/ex" {
            res.push_str(&format!("  {}{}:", "sql".bright_cyan(), &format_d(v.clone()))) ;
            match v["sql"].as_str() {
                Some(s) => res.push_str(&format!(" {}", s.cyan())),
                None => ()
            }
        } else if ev == "w/resp" {
            match v["w/st"].as_i64() {
                Some(s) => res.push_str(&format!(" {}: {}", "status".yellow(), if s < 399 { s.to_string().green() } else {s.to_string().red()})),
                None => ()
            }
            res.push_str(&format!("{}", format_d(v.clone())));
        } else if ev == "w/ex" {
            res.push_str(&format!("{}", " Exception:\n".red().bold()));
            if let Some(etr) = v["etr"].as_str() {
                res.push_str(&format!("{}",etr.bright_red()));
            } else {
                res.push_str(&format!("{}", v));
            }
        } else {
            let lvl = match v["lvl"].as_str() {
                Some(s) => s,
                None => "info"
            };

            let prev = if lvl == "warn" {
                ev.white().red()
            } else if lvl == "error" {
                ev.white().bright_red()
            } else {
                ev.white().bold()
            };
            res.push_str(&format!("    {}: {}", prev, v));
        }

        res.push_str("\n");
        return res;
    } else {
        return format!("UNK: {}", v);
    }
}

fn process_line(grp: &mut HashMap<String, String>, s:String ) -> () {
    let v: Value = serde_json::from_str(&s).unwrap();

    if v["ctx_end"].is_boolean() && v["ctx"].is_string() {
        let ctx = String::from(v["ctx"].as_str().unwrap()) ;
        match grp.get(&ctx) {
            Some(p) => print!("{}", p),
            None => ()
        };
        print!("{}", format_line(v));

    } else if v["ctx"].is_string() {
        let ctx = String::from(v["ctx"].as_str().unwrap()) ;
        let res:String = match grp.get(&ctx) {
            Some(p) => format!("{}{}", p, &format_line(v)),
            None => format_line(v)
        };
        grp.insert(ctx, res);

    } else {
        print!("{}", format_line(v));
    }
}

fn stdin_logs() {
    let stdin = io::stdin();

    let mut grp:HashMap<String, String>= HashMap::new();

    for line in stdin.lock().lines() {
        let s = line.unwrap();
        process_line(&mut grp, s);
    }
}

fn rest_logs(base_url:String, cl:String, sec:String) -> (){
    rt::run(rt::lazy(move || {
        let auth = base64::encode(&format!("{}:{}", cl, sec)); 
        let url = format!("{}/_logs", base_url);
        println!("Connecting to {}!", url);

        let client = Client::new();
        let req = hyper::Request::builder()
            .method("GET")
            .uri(url)
            .header("Authorization", format!("Basic {}", auth)).body(hyper::Body::default())
            .unwrap();


        client.request(req)
            .and_then(|res| {
                println!("Connected {}", res.status());
                let mut grp:HashMap<String, String>= HashMap::new();
                res.into_body()
                    .for_each(move |chunk| {
                        let mut v = Vec::new();
                        v.extend_from_slice(&*chunk);
                        process_line(&mut grp, String::from_utf8(v).unwrap());
                        return Ok(());
                    })
            }).map_err(|err| {
                println!("Error: {}", err);
            })

    }));
}

fn file_logs(f:String){

    let mut log_watcher = logwatcher::LogWatcher::register(f).unwrap();

    let mut grp:HashMap<String, String>= HashMap::new();

    log_watcher.watch(&mut |line: String| {
        process_line(&mut grp, line);
    });
}

fn main() {
    let logs_usage = "
read logs from API:
    aidbox -u http://mybox.url -c client-id -p client-secret logs

read logs stdin:
    tail -f logs | aidbox logs -i

read from file:
    aidbox logs -f logs
";
    let matches = App::new("aidbox cli")
        .version("0.0.1")
        .author("Health Samurai")
        .about("CLI tools for aidbox")
        .arg(Arg::with_name("url").short("u").value_name("BASE_URL").env("AIDBOX_BASE_URL").takes_value(true))
        .arg(Arg::with_name("client").short("c").value_name("CLIENT_ID").env("AIDBOX_CLIENT_SECRET").takes_value(true))
        .arg(Arg::with_name("secret").short("s").value_name("CLIENT_SECRET").env("AIDBOX_CLIENT_SECRET").takes_value(true))
        .subcommand(
            SubCommand::with_name("logs")
                .about("read aidbox logs")
                .usage(logs_usage)
                .arg(Arg::with_name("stdin").short("i"))
                .arg(Arg::with_name("file").short("f").env("LOGS_FILE").takes_value(true)))
        .subcommand(
            SubCommand::with_name("es")
                .about("Elasticsearch integration")
                .subcommand(
                    SubCommand::with_name("logs")
                        .about("load logs into es")
                        .arg(Arg::with_name("url").short("l").value_name("ES_URL").takes_value(true).required(true))
                        .arg(Arg::with_name("file").short("f").value_name("FILE").takes_value(true).required(true))
                )
        )
        .subcommand(
            SubCommand::with_name("pg")
                .about("Work with postgres")
                .arg(Arg::with_name("user").short("u").value_name("PGUSER").env("PGUSER").takes_value(true).required(true))
                .arg(Arg::with_name("password").short("w").value_name("PGPASSWORD").env("PGPASSWORD").takes_value(true).required(true))
                .arg(Arg::with_name("host").short("h").value_name("PGHOST").env("PGHOST").takes_value(true).required(true))
                .arg(Arg::with_name("port").short("p").value_name("PGPORT").env("PGPORT").takes_value(true).required(true))
                .arg(Arg::with_name("database").short("d").value_name("PGDATABASE").env("PGDATABASE").takes_value(true).required(true))
                .subcommand(
                    SubCommand::with_name("conn").about("test connection")
                )
                .subcommand(
                    SubCommand::with_name("logs").about("Load logs into postgres")
                        .arg(Arg::with_name("file").short("f").value_name("FILE").takes_value(true).required(true))
                        .arg(Arg::with_name("table").short("t").value_name("TABLE").takes_value(true).required(true))
                ))
        .get_matches();

    if let Some(logs_m) = matches.subcommand_matches("logs") {
        if let (Some(url), Some(cl), Some(sec)) = (matches.value_of("url"), matches.value_of("client"), matches.value_of("secret")) {
            rest_logs(String::from(url), String::from(cl), String::from(sec));
        } else if logs_m.is_present("stdin") {
          stdin_logs();
        } else if let Some(_file) = logs_m.value_of("file") {
            println!("Read from file {}", _file);
            file_logs(String::from(_file));
        } else {
            println!("Please provide BASE_URL; CLIENT_ID & CLIENT_SECRET");
        }
    } else if let Some(es_m) = matches.subcommand_matches("es") {
        if let Some(es_logs_m) = es_m.subcommand_matches("logs") {
            if let (Some(url), Some(_file)) = (es_logs_m.value_of("url"), es_logs_m.value_of("file"))
            {
                println!("Export to elasticsearch");
                esloader::load(_file.to_string(), url.to_string());
            } else {
                println!("Ups something missed");
            }
        }

    } else if let Some(pg_m) = matches.subcommand_matches("pg") {

        if let (Some(user), Some(password), Some(host), Some(port), Some(database))
            = (pg_m.value_of("user"), pg_m.value_of("password"), pg_m.value_of("host"), pg_m.value_of("port"), pg_m.value_of("database"))
        {

            let conn_cfg = pgloader::PgConn {
                user: String::from(user),
                password: String::from(password),
                host: String::from(host),
                port: String::from(port),
                database: String::from(database)
            };

            if let Some(l_m) = pg_m.subcommand_matches("logs") {
                if let (Some(file), Some(table)) = (l_m.value_of("file"), l_m.value_of("table")) {
                    pgloader::load(String::from(file),String::from(table), conn_cfg);
                } else {
                    println!("provide --file and --table");
                }
            } else if let Some(_l) = pg_m.subcommand_matches("conn") {
                println!("test conn");
            } else {
                println!("Unknown subcommand!");
            }
        } else {
            println!("Provide connection info");
        }
    }

}
