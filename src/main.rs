mod cli;
mod clock;
mod ntp;

use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::Parser;

use cli::*;
use clock::Clock;
use ntp::check_time;

fn main() {
    let args = Cli::parse();
    let action = args.get_action();
    let std = args.get_std();
    let datetime = args.get_datetime();

    match action {
        Action::Get => {
            let now = Clock::get();
            match std {
                TimeStandard::Rfc3339 => println!("RFC3339: {}", now.to_rfc3339()),
                TimeStandard::Rfc2822 => println!("RCF2822: {}", now.to_rfc2822()),
                TimeStandard::Timestamp => println!("{}", now.timestamp()),
            }
        }
        Action::Set => {
            let t_ = datetime.unwrap();
            let t = match std {
                TimeStandard::Rfc3339 => DateTime::parse_from_rfc3339(&t_),
                TimeStandard::Rfc2822 => DateTime::parse_from_rfc2822(&t_),
                _ => unimplemented!(),
            };
            let t = match t {
                Ok(t) => t,
                Err(_) => {
                    eprintln!("error: Unable to parse {} as {:?}", t_, std);
                    return;
                }
            };
            Clock::set(t);

            let maybe_error = std::io::Error::last_os_error();
            let os_error_code = maybe_error.raw_os_error();
            match os_error_code {
                Some(0) => (),
                Some(_) => eprintln!("Unable to set the time: {:?}", maybe_error),
                None => (),
            }
        }
        Action::CheckNtp => {
            let offset = check_time().unwrap() as isize;
            let adjust = Duration::from_millis(offset as u64);
            let now = if offset.is_positive() {
                Utc::now() + adjust
            } else {
                Utc::now() - adjust
            };
            let sign = if offset.is_positive() { "+" } else { "-" };
            println!("{now}  ({sign}{:?})", adjust);
        }
    }
}
