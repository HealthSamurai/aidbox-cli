// use std::env;
// use std::fs::File;
extern crate serde_json;
extern crate colored;

use std::io;
use std::io::prelude::*;
use serde_json::{Value};
use colored::*;
use std::collections::HashMap;


fn format_ts(ts: i64) -> String {

    // let millis = ts % 1000;
    let second = (ts / 1000) % 60;
    let minute = (ts / (1000 * 60)) % 60;
    let hour = (ts / (1000 * 60 * 60)) % 24;

    return format!("{:0>2}:{:0>2}:{:0>2}", hour, minute, second);
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

    if v["ev"].is_string() && v["ts"].is_number() {
        let ev = match v["ev"].as_str() {
            Some(s) => s,
            None => "Unknown"
        };

        match v["ts"].as_i64() {
            Some(ts) =>  res.push_str(&format!("{}",format_ts(ts).bright_black())),
            None => ()
        }
        if ev == "w/req" {
            let m = match v["w/m"].as_str() {
                Some(s) => s.to_uppercase().bold().yellow(),
                None => "GET".bold().yellow()
            };

            let url = match v["w/url"].as_str() {
                Some(s) => s.white().bold(),
                None => "???".white().bold()
            };

            res.push_str(&format!(" {} {}",m, url));

            match v["w/?"].as_str() {
                Some(qs) => res.push_str(&format!("?{}", qs)),
                None => ()
            }
            match v["w/addr"].as_str() {
                Some(s) => res.push_str(&format!(" by {}", s.bright_black())),
                None => ()
            }
        } else if ev == "db/q" || ev == "db/ex" {
            res.push_str(&format!("  {}{}:", "sql".bright_cyan(), &format_d(v.clone()))) ;
            match v["db/sql"].as_str() {
                Some(s) => res.push_str(&format!(" {}", s.cyan())),
                None => ()
            }
        } else if ev == "w/resp" {
            match v["w/st"].as_i64() {
                Some(s) => res.push_str(&format!(" {}: {}", "status".yellow(), if s < 399 { s.to_string().green() } else {s.to_string().red()})),
                None => ()
            }
            res.push_str(&format!("{}", format_d(v.clone())));
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

fn main() {
    let stdin = io::stdin();

    let mut grp:HashMap<String, String>= HashMap::new();

    for line in stdin.lock().lines() {

        let s = line.unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        if v["ctx!"].is_string() {
            let ctx = String::from(v["ctx!"].as_str().unwrap());
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
}
