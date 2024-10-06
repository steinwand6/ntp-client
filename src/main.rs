mod cli;
mod clock;

use std::{net::UdpSocket, time::Duration};

use byteorder::{BigEndian, ReadBytesExt};
use chrono::{DateTime, TimeZone, Timelike, Utc};
use clap::Parser;

use cli::*;
use clock::Clock;

const NTP_MESSAGE_LENGTH: usize = 48;
// Number of seconds between 1 Jan 1900(the NTP epoch) and 1 Jan 1970 (the UNIX epoch)
const NTP_TO_UNIX_SECONDS: i64 = 2_208_988_800;
const LOCAL_ADDR: &'static str = "0.0.0.0:12300";

#[derive(Debug, Default, Copy, Clone)]
struct NTPTimestamp {
    //NTP timestamps are expressed as 32-bit seconds and fractional fractions.
    seconds: u32,
    fraction: u32,
}

struct NTPMessage {
    data: [u8; NTP_MESSAGE_LENGTH],
}

struct NTPResult {
    // t1 is the local computer's record of the time when the first message is transmitted
    t1: DateTime<Utc>,
    // t2 is recorded by the remote server at the time that the first message is received.
    t2: DateTime<Utc>,
    // t3 is recorded by the remote server at the time that the second message is sent.
    t3: DateTime<Utc>,
    // t4 is the local computer's record of the time when the second message is received.
    t4: DateTime<Utc>,
}

impl NTPResult {
    pub fn delay(&self) -> i64 {
        // δ = (t4 - t1) - (t3 - t2)
        ((self.t4 - self.t1) - (self.t3 - self.t2)).num_milliseconds()
    }

    pub fn offset(&self) -> i64 {
        // θ = ((t2 – t1) + (t4 – t3)) / 2
        (((self.t2 - self.t1) + (self.t4 - self.t3)) / 2).num_milliseconds()
    }
}

impl From<NTPTimestamp> for DateTime<Utc> {
    fn from(ntp: NTPTimestamp) -> Self {
        let secs = ntp.seconds as i64 - NTP_TO_UNIX_SECONDS;
        let mut nanos = ntp.fraction as f64;
        nanos *= 1e9;
        nanos /= 2_f64.powi(32);

        Utc.timestamp_opt(secs, nanos as u32).unwrap()
    }
}

impl From<DateTime<Utc>> for NTPTimestamp {
    fn from(utc: DateTime<Utc>) -> Self {
        let secs = utc.timestamp() + NTP_TO_UNIX_SECONDS;
        let mut fraction = utc.nanosecond() as f64;
        fraction *= 2_f64.powi(32);
        fraction /= 1e9;

        NTPTimestamp {
            seconds: secs as u32,
            fraction: fraction as u32,
        }
    }
}

impl NTPMessage {
    fn new() -> Self {
        NTPMessage {
            data: [0; NTP_MESSAGE_LENGTH],
        }
    }

    fn client() -> Self {
        // 0 1 2 3 4 5 6 7 8
        // +-+-+-+-+-+-+-+-+
        // |LI | VN | MODE |
        const LEAP_INDICATOR: u8 = 0b_00_000_000;
        const VERSION: u8 = 0b_00_011_000; // version 3 is NTP (version 4 is SNTP)
        const MODE: u8 = 0b_00_000_011; // client mode

        let mut message = Self::new();
        message.data[0] |= LEAP_INDICATOR | VERSION | MODE;
        message
    }

    fn parse_timestamp(&self, i: usize) -> Result<NTPTimestamp, std::io::Error> {
        let mut reader = &self.data[i..i + 8];
        let seconds = reader.read_u32::<BigEndian>()?;
        let fraction = reader.read_u32::<BigEndian>()?;
        Ok(NTPTimestamp { seconds, fraction })
    }

    fn rx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        // t2
        self.parse_timestamp(32)
    }

    fn tx_time(&self) -> Result<NTPTimestamp, std::io::Error> {
        // t3
        self.parse_timestamp(40)
    }
}

/// This function calculates the weighted mean (average) of a set of values.
/// Each value has a weight, and values with higher weights have more influence on the result.
/// weights: A vector of weights, each corresponding to a value in the `values` vector.
/// values: A vector of values to calculate the weighted mean from.
fn weighted_mean(values: &Vec<f64>, weights: &Vec<f64>) -> f64 {
    let weighted_sum: f64 = values
        .iter()
        .zip(weights.iter())
        .map(|(value, weight)| value * weight)
        .sum();

    let total_weight: f64 = weights.iter().sum();

    weighted_sum / total_weight // Divide the weighted sum by the total of all weights
}

fn ntp_roundtrim(host: &str, port: u16) -> Result<NTPResult, std::io::Error> {
    let dest = format!("{}:{}", host, port);
    let timeout = Duration::from_secs(1);

    let request = NTPMessage::client();
    let mut response = NTPMessage::new();

    let udp = UdpSocket::bind(LOCAL_ADDR)?;
    udp.connect(dest).expect("Unable to connect");

    let t1 = Utc::now();
    udp.send(&request.data)?;
    udp.set_read_timeout(Some(timeout))?;
    udp.recv_from(&mut response.data)?;
    let t4 = Utc::now();

    let t2: DateTime<Utc> = response.rx_time().unwrap().into();
    let t3: DateTime<Utc> = response.tx_time().unwrap().into();

    Ok(NTPResult { t1, t2, t3, t4 })
}

fn check_time() -> Result<f64, std::io::Error> {
    const NTP_PORT: u16 = 123;
    let servers = [
        "time.nist.gov",
        "time.apple.com",
        "time.euro.apple.com",
        "time.google.com",
        "time2.google.com",
    ];

    let mut times = Vec::with_capacity(servers.len());

    for server in servers {
        print!("{} => ", server);

        match ntp_roundtrim(server, NTP_PORT) {
            Ok(time) => {
                println!("{}ms away from local system time", time.offset());
                times.push(time);
            }
            Err(_) => println!("? [response took too long]"),
        }
    }
    let mut offsets = Vec::with_capacity(times.len());
    let mut offset_weights = Vec::with_capacity(times.len());

    for time in &times {
        let offset = time.offset() as f64;
        let delay = time.delay() as f64;

        let weight = 1_000_000.0 / (delay * delay);
        if weight.is_finite() {
            offsets.push(offset);
            offset_weights.push(weight);
        }
    }

    let avg_offset = weighted_mean(&offsets, &offset_weights);
    Ok(avg_offset)
}

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
