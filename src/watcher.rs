use std::{thread};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::Write;

use chrono::prelude::*;
use reqwest::blocking::{Client};
use reqwest::Url;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Debug)]
pub struct HNTopStoriesSnap {
    t: DateTime<Utc>,
    items: Vec<HNTopStorySnap>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HNTopStorySnap {
    id: u32,

    #[serde(default)]
    #[serde(rename(serialize = "s"))]
    score: u32,

    #[serde(default)]
    #[serde(rename(serialize = "c"))]
    #[serde(rename(deserialize = "descendants"))]
    comments: u32,
}

pub enum EventType {
    Interrupted,
    AlarmFired
}

pub struct HNWatcher {
    http_c : Client,
    base_url : Url,
    out_prefix : String,
    out_f: Option<File>,
    tx: Sender<EventType>,
    rx: Receiver<EventType>,
}

impl HNWatcher {

    pub fn new(base_url : &str, com: (Sender<EventType>, Receiver<EventType>)) -> Self {

        HNWatcher {
            http_c : Client::new(),
            base_url: Url::parse(base_url).unwrap(),
            out_prefix: String::from("data"),
            out_f: None,
            tx: com.0,
            rx: com.1,
        }
    }

    pub fn watch(&mut self, watch_int: u64) {
        let sleep_t = Duration::from_secs(watch_int);
        let start_day = Utc::today();

        self.open_output_file(&format!("{}-{}.jl", self.out_prefix, start_day));

        // Send an initial event to start the download
        self.tx.send(EventType::AlarmFired).unwrap();
        // Now we can start the loop
        loop {
            // Block until next event
            let msg = self.rx.recv().unwrap();
            match msg {
                EventType::Interrupted => {
                    break;
                },
                EventType::AlarmFired => {
                    let t = Utc::now();
                    let snap = self.get_top_stories_snap(t);
                    let serialized_snap = serde_json::to_string(&snap).unwrap();
                    if t.date() > start_day {
                        self.handle_out_file_rotation(t);
                    }
                    self.write_snap_to_file(&serialized_snap);
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
                println!("Sleeping for {} secs", sleep_t.as_secs());
                thread::sleep(sleep_t);
                tx.send(EventType::AlarmFired).unwrap();
            })
            .expect("Failed to spawn thread with name");
    }

    fn get_top_stories_snap(&self, t: DateTime<Utc>) -> HNTopStoriesSnap {
        let ids = self.get_top_stories_ids();
        // Select only the first 30 items (the first page) and get stats
        let items: Vec<HNTopStorySnap> = ids.into_iter().take(30).map(|id| {
            self.get_story_detail_snap(id)
        }).collect();

        HNTopStoriesSnap {t, items}
    }

    fn get_top_stories_ids(&self) -> Vec<u32> {
        let url = self.base_url.join("topstories.json").unwrap();
        let resp = self.http_c.get(url).send().unwrap();
        resp.json::<Vec<u32>>().unwrap()
    }

    fn get_story_detail_snap(&self, id: u32) -> HNTopStorySnap {
        let url = self.base_url.join(&format!("item/{}.json", id)).unwrap();
        let resp = self.http_c.get(url).send().unwrap();
        resp.json::<HNTopStorySnap>().unwrap()
    }

    fn handle_out_file_rotation(&mut self, t: DateTime<Utc>) {
        self.open_output_file(&format!("{}-{}.jl", self.out_prefix, t.date()));
    }

    fn open_output_file(&mut self, name: &str) {
        let f = OpenOptions::new().create(true).append(true)
                                    .open(Path::new(name)).unwrap();
        self.out_f = Some(f);
    }

    fn write_snap_to_file(&mut self, snap: &str) {
        writeln!((&mut self.out_f.as_mut().unwrap()), "{}", snap).unwrap();
    }
}