use byteorder::{BigEndian, ReadBytesExt};
use chrono::{DateTime, TimeZone, Timelike, Utc};

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

fn main() {
    println!("Hello, world!");
}
