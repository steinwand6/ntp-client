use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "clock",
    version = "0.1",
    about = "Gets and (aspirationally) sets the time."
)]
pub struct Cli {
    // Action to perform: get or set
    #[arg(default_value = "get")]
    action: Action,
    // Time standard to use for output
    #[arg(short, long = "use-standard", default_value = "rfc3339")]
    std: TimeStandard,
    // Datetime value, used when the action is "set"
    #[arg()]
    datetime: Option<String>,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum Action {
    Get,
    Set,
    CheckNtp,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum TimeStandard {
    Rfc3339,
    Rfc2822,
    Timestamp,
}

impl Cli {
    pub fn get_action(&self) -> &Action {
        &self.action
    }

    pub fn get_std(&self) -> &TimeStandard {
        &self.std
    }

    pub fn get_datetime(&self) -> Option<&str> {
        match &self.datetime {
            Some(datetime) => Some(&datetime),
            None => None,
        }
    }
}
