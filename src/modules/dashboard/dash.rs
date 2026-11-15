use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use amane::{Center, Color, Column, Image, Parent, Rectangle, Row, Service, Start, Text, Widget};

use crate::theme::ThemeColors;

const CONTENT_WIDTH: f32 = 694.0;
const CONTENT_HEIGHT: f32 = 390.0;
const LEFT_COLUMN_WIDTH: f32 = 240.0;
const CALENDAR_WIDTH: f32 = 438.0;
const CARD_GAP: f32 = 16.0;

#[derive(Clone, PartialEq, Eq)]
struct DateClock {
    time: String,
    weekday: String,
    day: u32,
    month_name: String,
    year: i32,
    first_weekday: u32,
    days_in_month: u32,
}

impl Default for DateClock {
    fn default() -> Self {
        Self {
            time: "--:--".into(),
            weekday: String::new(),
            day: 1,
            month_name: String::new(),
            year: 1970,
            first_weekday: 4,
            days_in_month: 31,
        }
    }
}

impl Service for DateClock {
    fn new() -> Self {
        read_clock()
    }

    fn interval() -> Duration {
        Duration::from_secs(10)
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

fn read_clock() -> DateClock {
    let output = Command::new("date")
        .arg("+%H:%M|%A|%d|%B|%m|%Y|%w")
        .output();
    let Ok(output) = output else {
        return DateClock::default();
    };
    if !output.status.success() {
        return DateClock::default();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = text.trim().split('|').collect();
    if fields.len() != 7 {
        return DateClock::default();
    }

    let day = fields[2].parse::<u32>().unwrap_or(1).clamp(1, 31);
    let month = fields[4].parse::<u32>().unwrap_or(1).clamp(1, 12);
    let year = fields[5].parse::<i32>().unwrap_or(1970);
    let weekday_index = fields[6].parse::<u32>().unwrap_or(0).min(6);
    let days_in_month = days_in_month(year, month);
    let first_weekday = (weekday_index + 7 - ((day - 1) % 7)) % 7;

    DateClock {
        time: fields[0].to_owned(),
        weekday: fields[1].to_owned(),
        day,
        month_name: fields[3].to_owned(),
        year,
        first_weekday,
        days_in_month,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 30,
    }
}

pub(super) fn content_size() -> (f32, f32) {
    (CONTENT_WIDTH, CONTENT_HEIGHT)
}

pub(super) fn view(theme: ThemeColors) -> Rectangle {
    let clock = DateClock::read();
    let title = format!("{} {}", capitalize(&clock.month_name), clock.year);
    let user = env::var("USER").unwrap_or_else(|_| "Usuário".to_owned());

    let profile = profile_card(&user, theme);
    let clock_card = clock_card(&clock, theme);
    let left_column = Column::new(vec![Box::new(profile), Box::new(clock_card)])
        .width(LEFT_COLUMN_WIDTH)
        .height(CONTENT_HEIGHT)
        .gap(CARD_GAP)
        .align(Start)
        .justify(Start);

    let calendar = calendar_card(&clock, &title, theme);

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(CONTENT_HEIGHT)
        .fill(Color::TRANSPARENT)
        .child(
            Row::new(vec![Box::new(left_column), Box::new(calendar)])
                .width(CONTENT_WIDTH)
                .height(CONTENT_HEIGHT)
                .gap(CARD_GAP)
                .align(Start)
                .justify(Start),
        )
}

fn profile_card(user: &str, theme: ThemeColors) -> Rectangle {
    const AVATAR_SIZE: f32 = 112.0;

    let avatar = profile_image().unwrap_or_else(|| {
        Rectangle::new()
            .width(AVATAR_SIZE)
            .height(AVATAR_SIZE)
            .radius(AVATAR_SIZE / 2.0)
            .fill(theme.active_surface)
            .align_child(Center, Center)
            .child(
                Text::new("\u{e7fd}")
                    .font("Material Symbols Rounded")
                    .size(54.0)
                    .color(theme.active_text),
            )
    });

    Rectangle::new()
        .width(LEFT_COLUMN_WIDTH)
        .height(180.0)
        .radius(24.0)
        .fill(theme.button_surface)
        .padding(16.0)
        .child(
            Row::new(vec![
                Box::new(avatar),
                Box::new(
                    Column::new(vec![
                        Box::new(Text::new("Bem-vindo").size(12.0).color(theme.muted)),
                        Box::new(
                            Text::new(user.to_owned())
                                .size(18.0)
                                .color(theme.text)
                                .elide(),
                        ),
                        Box::new(Text::new("Sua sessão").size(11.0).color(theme.muted)),
                    ])
                    .width(84.0)
                    .height(Parent)
                    .gap(7.0)
                    .align(Start)
                    .justify(Center),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(12.0)
            .align(Center)
            .justify(Start),
        )
}

fn clock_card(clock: &DateClock, theme: ThemeColors) -> Rectangle {
    let (hour, minute) = clock.time.split_once(':').unwrap_or((&clock.time, "--"));

    Rectangle::new()
        .width(LEFT_COLUMN_WIDTH)
        .height(194.0)
        .radius(24.0)
        .fill(theme.button_surface)
        .padding(18.0)
        .child(
            Column::new(vec![
                Box::new(Text::new(hour.to_owned()).size(43.0).color(theme.text)),
                Box::new(Text::new("•••").size(13.0).color(theme.accent)),
                Box::new(Text::new(minute.to_owned()).size(43.0).color(theme.text)),
                Box::new(
                    Text::new(format!("{} · {}", capitalize(&clock.weekday), clock.day))
                        .size(13.0)
                        .color(theme.muted),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(1.0)
            .align(Start)
            .justify(Center),
        )
}

fn profile_image() -> Option<Rectangle> {
    const AVATAR_SIZE: f32 = 112.0;
    let home = env::var_os("HOME").map(PathBuf::from)?;
    let candidates = [home.join(".face"), home.join(".face.icon")];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| {
            Rectangle::new()
                .width(AVATAR_SIZE)
                .height(AVATAR_SIZE)
                .radius(AVATAR_SIZE / 2.0)
                .clip()
                .fill(Image::cover(path))
        })
}

fn calendar_card(clock: &DateClock, title: &str, theme: ThemeColors) -> Rectangle {
    const CELL_WIDTH: f32 = 55.0;
    const CELL_HEIGHT: f32 = 40.0;
    const GRID_GAP: f32 = 2.0;

    let mut rows: Vec<Box<dyn Widget>> = Vec::new();
    let weekdays = ["D", "S", "T", "Q", "Q", "S", "S"];
    let weekday_cells = weekdays
        .iter()
        .map(|day| {
            Box::new(
                Rectangle::new()
                    .width(CELL_WIDTH)
                    .height(24.0)
                    .align_child(Center, Center)
                    .child(Text::new(*day).size(12.0).color(theme.muted)),
            ) as Box<dyn Widget>
        })
        .collect();
    rows.push(Box::new(
        Row::new(weekday_cells)
            .width(Parent)
            .height(24.0)
            .gap(GRID_GAP)
            .align(Center)
            .justify(Start),
    ));

    for week in 0..6 {
        let mut cells: Vec<Box<dyn Widget>> = Vec::new();
        for column in 0..7 {
            let slot = week * 7 + column;
            let day = slot as i32 - clock.first_weekday as i32 + 1;
            if day < 1 || day > clock.days_in_month as i32 {
                cells.push(Box::new(
                    Rectangle::new().width(CELL_WIDTH).height(CELL_HEIGHT),
                ));
                continue;
            }

            let is_today = day as u32 == clock.day;
            let cell = Rectangle::new()
                .width(CELL_WIDTH)
                .height(CELL_HEIGHT)
                .radius(13.0)
                .fill(if is_today {
                    theme.active_surface
                } else {
                    Color::TRANSPARENT
                })
                .align_child(Center, Center)
                .child(Text::new(day.to_string()).size(14.0).color(if is_today {
                    theme.active_text
                } else {
                    theme.text
                }));
            cells.push(Box::new(cell));
        }
        rows.push(Box::new(
            Row::new(cells)
                .width(Parent)
                .height(CELL_HEIGHT)
                .gap(GRID_GAP)
                .align(Center)
                .justify(Start),
        ));
    }

    Rectangle::new()
        .width(CALENDAR_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(28.0)
        .fill(theme.button_surface)
        .padding(16.0)
        .child(
            Column::new(vec![
                Box::new(Text::new(title.to_owned()).size(19.0).color(theme.text)),
                Box::new(
                    Column::new(rows)
                        .width(Parent)
                        .height(288.0)
                        .gap(4.0)
                        .align(Start)
                        .justify(Start),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(12.0)
            .align(Start)
            .justify(Center),
        )
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().collect::<String>() + chars.as_str()
}
