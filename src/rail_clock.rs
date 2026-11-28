use std::process::Command;
use std::time::Duration;

use amane::Service;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RailClock {
    pub day_month: String,
    pub time: String,
}

impl RailClock {
    pub fn snapshot() -> Self {
        Self::read().clone()
    }
}

impl Service for RailClock {
    fn new() -> Self {
        read_clock()
    }

    fn interval() -> Duration {
        Duration::from_secs(1)
    }

    fn update(&mut self) -> bool {
        let next = read_clock();
        if *self == next {
            return false;
        }

        *self = next;
        true
    }
}

fn read_clock() -> RailClock {
    const MONTHS: [&str; 12] = [
        "JAN", "FEV", "MAR", "ABR", "MAI", "JUN", "JUL", "AGO", "SET", "OUT", "NOV", "DEZ",
    ];

    let output = Command::new("date").arg("+%d|%m|%H:%M").output();
    let Ok(output) = output else {
        return RailClock {
            day_month: "-- ---".to_owned(),
            time: "--:--".to_owned(),
        };
    };
    if !output.status.success() {
        return RailClock {
            day_month: "-- ---".to_owned(),
            time: "--:--".to_owned(),
        };
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = text.trim().split('|').collect();
    if fields.len() != 3 {
        return RailClock {
            day_month: "-- ---".to_owned(),
            time: "--:--".to_owned(),
        };
    }

    let day = fields[0].parse::<u32>().unwrap_or(0);
    let month = fields[1].parse::<usize>().unwrap_or(0);
    let day_month = match month
        .checked_sub(1)
        .and_then(|month_index| MONTHS.get(month_index))
    {
        Some(month_name) if (1..=31).contains(&day) => format!("{day:02} {month_name}"),
        _ => "-- ---".to_owned(),
    };

    RailClock {
        day_month,
        time: fields[2].to_owned(),
    }
}
