use std::sync::mpsc;

extern crate ctrlc;
mod watcher;


fn main() {
    let (tx, rx) = mpsc::channel::<watcher::EventType>();

    // Set-up handling of SIGINT
    let com_channel = tx.clone();
    ctrlc::set_handler(move || {
        com_channel.send(watcher::EventType::Interrupted).ok();
    }).expect("Error setting Ctrl-C handler");

    // Init watcher
    let hn_watcher = watcher::HNWatcher::new(
        "https://hacker-news.firebaseio.com/v0/",
        (tx, rx)
    );

    hn_watcher.watch(5);
}
