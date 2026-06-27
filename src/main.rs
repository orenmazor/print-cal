use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const API_URL: &str = "https://www.googleapis.com/calendar/v3";
const CALENDARS_URL: &str = "https://www.googleapis.com/calendar/v3/users/me/calendarList";

#[derive(Debug)]
struct Config {
    days: i64,
    output: Option<PathBuf>,
    client_id: String,
    client_secret: String,
    refresh_token: String,
    list_calendars: bool,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct EventsResponse {
    #[serde(default)]
    items: Vec<GoogleEvent>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct CalendarsResponse {
    #[serde(default)]
    items: Vec<Calendar>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Calendar {
    id: String,
    summary: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct GoogleEvent {
    id: String,
    #[serde(default)]
    summary: String,
    start: EventTime,
    end: EventTime,
    #[serde(default)]
    location: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct EventTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Serialize)]
struct OutputEvent {
    id: String,
    title: String,
    start: String,
    end: String,
    location: String,
    description: String,
    all_day: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = config()?;
    let client = Client::new();
    let access_token = refresh_access_token(&client, &config)?;
    if config.list_calendars {
        return print_calendars(&client, &access_token);
    }

    let mut events = Vec::new();
    for calendar in fetch_calendars(&client, &access_token)? {
        events.extend(fetch_events(&client, &access_token, &calendar.id, config.days)?);
    }
    events.sort_by(|a, b| a.start.cmp(&b.start));
    write_json(config.output.as_ref(), &events)
}

fn config() -> Result<Config> {
    let mut days = 3;
    let mut output = None;
    let mut list_calendars = false;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--days" => {
                days = args
                    .next()
                    .ok_or_else(|| anyhow!("--days requires a value"))?
                    .parse()
                    .context("--days must be a number")?;
            }
            "--output" => output = Some(PathBuf::from(args.next().ok_or_else(|| anyhow!("--output requires a path"))?)),
            "--list-calendars" => list_calendars = true,
            "--help" | "-h" => {
                println!("Usage: print-cal [--days N] [--output PATH] [--list-calendars]");
                std::process::exit(0);
            }
            _ => return Err(anyhow!("unknown argument: {arg}")),
        }
    }

    if days < 0 {
        return Err(anyhow!("--days must be non-negative"));
    }

    Ok(Config {
        days,
        output,
        client_id: env::var("GOOGLE_CLIENT_ID").context("GOOGLE_CLIENT_ID is not set")?,
        client_secret: env::var("GOOGLE_CLIENT_SECRET").context("GOOGLE_CLIENT_SECRET is not set")?,
        refresh_token: env::var("GOOGLE_REFRESH_TOKEN").context("GOOGLE_REFRESH_TOKEN is not set")?,
        list_calendars,
    })
}

fn refresh_access_token(client: &Client, config: &Config) -> Result<String> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", config.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .context("failed to refresh Google access token")?;

    if !response.status().is_success() {
        return Err(anyhow!("Google token refresh failed: {}", response.text()?));
    }

    Ok(response.json::<TokenResponse>()?.access_token)
}

fn print_calendars(client: &Client, access_token: &str) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&fetch_calendars(client, access_token)?)?);
    Ok(())
}

fn fetch_calendars(client: &Client, access_token: &str) -> Result<Vec<Calendar>> {
    let mut out = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut query = vec![("maxResults", "250")];
        if let Some(token) = &page_token {
            query.push(("pageToken", token.as_str()));
        }

        let response = client
            .get(CALENDARS_URL)
            .bearer_auth(access_token)
            .query(&query)
            .send()
            .context("failed to fetch Google calendars")?;

        if !response.status().is_success() {
            return Err(anyhow!("Google Calendar list failed: {}", response.text()?));
        }

        let page = response.json::<CalendarsResponse>()?;
        out.extend(page.items);
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(out);
        }
    }
}

fn fetch_events(client: &Client, access_token: &str, calendar_id: &str, days: i64) -> Result<Vec<OutputEvent>> {
    let now = Utc::now();
    let time_min = now.to_rfc3339();
    let time_max = (now + Duration::days(days)).to_rfc3339();
    let mut out = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut query = vec![
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
            ("maxResults", "2500"),
            ("timeMin", time_min.as_str()),
            ("timeMax", time_max.as_str()),
        ];
        if let Some(token) = &page_token {
            query.push(("pageToken", token.as_str()));
        }

        let url = format!(
            "{API_URL}/calendars/{}/events",
            urlencoding::encode(calendar_id)
        );
        let response = client
            .get(url)
            .bearer_auth(access_token)
            .query(&query)
            .send()
            .context("failed to fetch Google Calendar events")?;

        if !response.status().is_success() {
            return Err(anyhow!("Google Calendar fetch failed: {}", response.text()?));
        }

        let page = response.json::<EventsResponse>()?;
        out.extend(
            page.items
                .into_iter()
                .map(OutputEvent::try_from)
                .collect::<Result<Vec<_>>>()?,
        );
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(out);
        }
    }
}

impl TryFrom<GoogleEvent> for OutputEvent {
    type Error = anyhow::Error;

    fn try_from(event: GoogleEvent) -> Result<Self> {
        Ok(Self {
            id: event.id,
            title: event.summary,
            start: event.start.iso()?,
            end: event.end.iso()?,
            location: event.location,
            description: event.description,
            all_day: event.start.date.is_some(),
        })
    }
}

impl EventTime {
    fn iso(&self) -> Result<String> {
        if let Some(date_time) = &self.date_time {
            return Ok(DateTime::parse_from_rfc3339(date_time)
                .with_context(|| format!("invalid event dateTime: {date_time}"))?
                .with_timezone(&Utc)
                .to_rfc3339());
        }

        self.date
            .clone()
            .ok_or_else(|| anyhow!("event time has neither dateTime nor date"))
    }
}

fn write_json(path: Option<&PathBuf>, events: &[OutputEvent]) -> Result<()> {
    let json = serde_json::to_vec_pretty(events)?;
    let Some(path) = path else {
        println!("{}", String::from_utf8(json)?);
        return Ok(());
    };

    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    }

    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("json")
    ));

    fs::write(&tmp, json).with_context(|| format!("failed to write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("failed to move {} to {}", tmp.display(), path.display()))?;
    Ok(())
}
