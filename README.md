# print-cal

`print-cal` is a small Rust CLI that fetches upcoming events from all calendars in your Google Calendar account and prints them as JSON.

It is designed for widgets such as [EWW](https://github.com/elkowar/eww), cron jobs, or systemd timers: it runs once, writes/prints JSON, and exits.

![Example EWW calendar widget](example.png)

## Features

- Fetches events from all available Google calendars
- Expands recurring events
- Sorts events by start time
- Prints JSON to stdout by default
- Optionally writes JSON atomically to a file
- Uses Google Calendar REST API directly
- No daemon, no local database, no Google SDK

## Build

```sh
cargo build --release
```

With [Task](https://taskfile.dev/):

```sh
task
```

The binary will be at:

```sh
target/release/print-cal
```

## Google setup

You need a Google OAuth desktop client and a refresh token.

### 1. Create OAuth credentials

In Google Cloud Console:

1. Create/select a project
2. Enable **Google Calendar API**
3. Configure OAuth consent screen
4. Add your Google account as a test user if the app is in testing mode
5. Create OAuth client credentials
6. Application type: **Desktop app**

Save:

```sh
GOOGLE_CLIENT_ID
GOOGLE_CLIENT_SECRET
```

### 2. Get a refresh token

This repo includes a Taskfile helper:

```sh
export GOOGLE_CLIENT_ID="..."
export GOOGLE_CLIENT_SECRET="..."
task auth-token
```

Open the printed URL, approve access, copy the returned `code=...`, and paste it into the prompt.

Save the printed refresh token as:

```sh
GOOGLE_REFRESH_TOKEN
```

## Environment variables

Required:

```sh
export GOOGLE_CLIENT_ID="..."
export GOOGLE_CLIENT_SECRET="..."
export GOOGLE_REFRESH_TOKEN="..."
```

The refresh token is sensitive. Do not commit it.

## Usage

Fetch the next 3 days of events and print JSON:

```sh
print-cal
```

Fetch the next 5 days:

```sh
print-cal --days 5
```

Write JSON atomically to a file:

```sh
print-cal --days 5 --output events.json
```

List calendars visible to your account:

```sh
print-cal --list-calendars
```

## JSON output

```json
[
  {
    "id": "...",
    "title": "Team sync",
    "start": "2026-06-27T10:00:00+00:00",
    "end": "2026-06-27T10:30:00+00:00",
    "location": "Office",
    "description": "...",
    "all_day": false
  }
]
```

## EWW example

One use is a desktop calendar widget that polls `print-cal` and displays the next few events.

Example script:

```sh
#!/bin/sh
set -a
. "$HOME/.config/print-cal/env"
set +a

exec /path/to/print-cal --days 5
```

Example EWW poll:

```lisp
(defpoll calendar :interval "10m" "~/.config/eww/scripts/calendar")
```

The screenshot above (`example.png`) shows a compact macOS-style EWW widget built from this output.

## Secret storage

Avoid putting credentials in:

- `.yuck` files
- public dotfiles
- Taskfile commands
- shell history
- CLI arguments

Simple local setup:

```sh
mkdir -p ~/.config/print-cal
chmod 700 ~/.config/print-cal
$EDITOR ~/.config/print-cal/env
chmod 600 ~/.config/print-cal/env
```

`~/.config/print-cal/env`:

```sh
GOOGLE_CLIENT_ID='...'
GOOGLE_CLIENT_SECRET='...'
GOOGLE_REFRESH_TOKEN='...'
```

For no plaintext secrets on disk, use `pass`, `secret-tool`, KeePassXC Secret Service, or systemd user services with encrypted credentials.

## Examples

### Generate JSON for EWW

```sh
print-cal --days 5 --output /tmp/eww-calendar.json
```

### Use in cron

```cron
*/10 * * * * GOOGLE_CLIENT_ID=... GOOGLE_CLIENT_SECRET=... GOOGLE_REFRESH_TOKEN=... /usr/local/bin/print-cal --days 5 --output /tmp/calendar.json
```

Prefer sourcing secrets from a private env file instead of putting them directly in cron.

### Inspect events with jq

```sh
print-cal --days 14 | jq '.[] | {title, start, calendar: .calendar}'
```

Note: current output does not include calendar name/id, only event fields.

## Development

```sh
cargo check
cargo test
cargo run -- --days 5
```
