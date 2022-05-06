use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

struct PassTimings {
    current_pass_name: String,
    current_pass_start_time: Instant,
    all_passes: Vec<PassDuration>,
}

#[derive(Clone)]
struct PassDuration {
    pass_name: String,
    duration: Duration,
}

static TIME_PASSES: AtomicBool = AtomicBool::new(false);

thread_local! {
    static PASSES: RefCell<Option<PassTimings>> = RefCell::new(None);
}

/// Set whether the time! macro should print out the timings of each pass or not
pub fn time_passes(should_time: bool) {
    TIME_PASSES.store(should_time, Ordering::Relaxed);
}

/// Start timing the given pass
pub fn start_time(pass_name: &str) {
    if TIME_PASSES.load(Ordering::Relaxed) {
        PASSES.with(|passes| {
            let mut passes = passes.borrow_mut();
            match passes.as_mut() {
                Some(time) => time.all_passes.push(time.current_pass_duration()),
                None => *passes = Some(PassTimings::new()),
            };
            let time = passes.as_mut().unwrap();
            time.current_pass_name = pass_name.to_string();
            time.current_pass_start_time = Instant::now();
        })
    }
}

pub fn show_timings() {
    if TIME_PASSES.load(Ordering::Relaxed) {
        PASSES.with(|timings| {
            let mut timings = timings.borrow_mut();
            let timings = timings.as_mut().unwrap();
            let final_pass = timings.current_pass_duration();
            timings.all_passes.push(final_pass);

            timings.all_passes.iter().for_each(|pass| pass.show());

            println!("{}\nTotals:\n", "-".repeat(33));

            let aggregate = timings.aggregate_pass_timings();
            aggregate.iter().for_each(|pass| pass.show());
        })
    }
}

impl PassTimings {
    fn new() -> PassTimings {
        PassTimings { current_pass_name: String::new(), current_pass_start_time: Instant::now(), all_passes: vec![] }
    }

    fn current_pass_duration(&self) -> PassDuration {
        PassDuration { pass_name: self.current_pass_name.to_string(), duration: self.current_pass_start_time.elapsed() }
    }

    /// Combine the Durations of all passes with the same name
    fn aggregate_pass_timings(&self) -> Vec<PassDuration> {
        let mut aggregate: Vec<PassDuration> = vec![];

        for pass in self.all_passes.iter() {
            match aggregate.iter_mut().find(|previous| previous.pass_name == pass.pass_name) {
                Some(previous) => previous.duration += pass.duration,
                None => aggregate.push(pass.clone()),
            }
        }

        aggregate
    }
}

impl PassDuration {
    fn show(&self) {
        let millis = self.duration.as_millis();

        let time_string =
            if millis != 0 { format!("{}ms", millis) } else { format!("{}μs", self.duration.as_micros()) };

        println!("{: <25} - {}", self.pass_name, time_string);
    }
}
