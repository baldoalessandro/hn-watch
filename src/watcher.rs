use std::{thread};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};

use reqwest::blocking::{Client};
use reqwest::Url;


pub enum EventType {
    Interrupted,
    AlarmFired
}

pub struct HNWatcher {
    http_c : Client,
    base_url : Url,
    tx: Sender<EventType>,
    rx: Receiver<EventType>,
}

impl HNWatcher {

    pub fn new(base_url : &str, com: (Sender<EventType>, Receiver<EventType>)) -> Self {
        HNWatcher {
            http_c : Client::new(),
            base_url: Url::parse(base_url).unwrap(),
            tx: com.0,
            rx: com.1,
        }
    }

    pub fn watch(&self, watch_int: u64) {
        let sleep_t = Duration::from_secs(watch_int);

        // Send an initial event to start the download
        self.tx.send(EventType::AlarmFired).unwrap();
        loop {
            // Block until next event
            let msg = self.rx.recv().unwrap();
            match msg {
                EventType::Interrupted => {
                    break;
                },
                EventType::AlarmFired => {
                    let ids = self.get_topstories_ids();
                    println!("{:#?}", ids);
                    // Re-schedule this event after some interval
                    self.sleep_thread(sleep_t);
                }
            }
        }
    }

    fn sleep_thread(&self, sleep_t: Duration) {
        let tx = self.tx.clone();
        thread::Builder::new()
            .name("hn-watcher-sleep".into())
            .spawn(move || {
                println!("Sleeping for 5 secs");
                thread::sleep(sleep_t);
                tx.send(EventType::AlarmFired).unwrap();
            })
            .expect("Failed to spawn thread with name");
    }

    fn get_topstories_ids(&self) -> Vec<u32> {
        let url = self.base_url.join("topstories.json").unwrap();
        let resp = self.http_c.get(url).send().unwrap();
        resp.json::<Vec<u32>>().unwrap()
    }
}