extern crate threadpool;
extern crate reqwest;

use regex::Regex;
use std::io::{BufReader, BufRead};
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::{thread, time};
use threadpool::ThreadPool;

fn publish_file(filename: &str, attempts: u8) {
    let client = reqwest::Client::new();
    let uploaded = reqwest::Body::from(std::fs::File::open(filename).unwrap());

    println!("Start [{}]", filename);    
    let put_request = client.put("http://oss.sonatype.org/somewhere/")
        .basic_auth("username", Some("password"))
        .body(uploaded)
        .send();
    
    match put_request {
        Ok(_)  => {
            println!("Done [{}]", filename);
        },
        Err(_) => {
            //println!("{:?}", err);
            if attempts > 0 {             
                eprintln!("Failed [{}], waiting 5 second then trying again", filename);            
                thread::sleep(time::Duration::from_secs(5));
                publish_file(filename, attempts - 1);
            } else {
                std::process::exit({
                    eprintln!("Repeatedly failed [{}], giving up", filename);
                    1
                })
            }
        }
    }
}

fn main() {

    // START SBT PROCESS
    // ====================================================================================
    // Build project
    // let mut cmd = Command::new("sbt")
    //     .arg("-mem").arg("4096")
    //     .arg("+test")
    //     .arg("+signedArtifacts")        
    //     .stdout(Stdio::piped())
    //     .spawn()
    //     .expect("failed to test and build");
    
    let mut cmd = Command::new("cat")
        .arg("/tmp/sbtout")
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to test and build");
    
    println!("Building...");    

    let n_workers = 4;
    
    let pool = ThreadPool::new(n_workers);
    let (tx, rx) = channel();
    let re = Regex::new(r"^.*Packaging\s*(?P<filename>.*?) [.]{3}.*$").unwrap();
    let mut publish_files = Vec::new();    
    {
        let stdout = cmd.stdout.as_mut().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        for line in stdout_lines {
            let line = line.expect("unable to read line");

            match re.captures(&line) {
                Some(caps) => {                    
                    let filename = format!("{}", &caps["filename"]);
                    let f = filename.clone();
                    let tx = tx.clone();
                    pool.execute(move || {
                        publish_file(&filename, 10);
                        tx.send(filename).expect("channel will be there waiting for the pool");
                    });

                    publish_files.push(f);
                },
                None => {}
            }
        }
    }
    
    cmd.wait().expect("failed to build artifacts");
    let n_jobs = publish_files.len();
    println!("{} files found", rx.iter().take(n_jobs).fold(0, |a, _| a + 1));
}
