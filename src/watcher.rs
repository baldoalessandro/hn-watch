use std::{thread};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};

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
                    let snap = self.get_top_stories_snap();
                    let serialized_snap = serde_json::to_string(&snap).unwrap();
                    println!("{:#?}", serialized_snap);
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

    fn get_top_stories_snap(&self) -> HNTopStoriesSnap {
        let t = Utc::now();
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
}