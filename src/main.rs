#![cfg_attr(not(test), windows_subsystem = "windows")]

mod eventlog;
mod fonts;
mod model;
mod time;
mod theme;
mod ui;
mod win;

fn main() {
    if std::env::args().any(|a| a == "--licenses") {
        println!("PC Time\n{}\n{}\nPretendard\n{}", include_str!("../LICENSE"),theme::NOTICE,fonts::NOTICE);
        return;
    }
    if std::env::args().any(|a| a == "--report") {
        match eventlog::read() {
            Ok(log) => {
                let now = time::now();
                let history = model::reconstruct(&log.events, now);
                println!("PC TIME | System event log | {} events | {} unparsed", log.count, log.skipped);
                println!("Updated: {}", time::stamp(now));
                for day in model::days(&history, now, 30) {
                    println!("{} | {} | {} segments | {} uncertain", time::date(day.start), model::duration(day.seconds), day.count, day.uncertain);
                }
                for s in history.iter().rev().take(25) {
                    println!("{} -> {} | {} | {}", time::stamp(s.start), s.end.map(time::stamp).unwrap_or_else(|| "?".into()), s.seconds().map(model::duration).unwrap_or_else(|| "excluded".into()), s.label());
                }
            }
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        }
    } else {
        ui::run();
    }
}
