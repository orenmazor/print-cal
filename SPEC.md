# print-cal

## Goal

A small CLI that downloads upcoming Google Calendar events and writes them to a JSON file for EWW.

## Requirements

- Runs once and exits.
- No daemon.
- Designed to be called from cron/systemd/manual.
- Small memory footprint.
- Fast startup (<200 ms excluding network).

## Inputs

Environment variables:

GOOGLE_CLIENT_ID
GOOGLE_CLIENT_SECRET
GOOGLE_REFRESH_TOKEN

CLI flags:

--days <N>        default 3
--output <path>   required

## Behavior

1. Refresh OAuth access token.
2. Query the primary Google Calendar.
3. Fetch events between now and now + N days.
4. Expand recurring events.
5. Sort by start time.
6. Write JSON atomically.

## JSON format

[
  {
    "id": "...",
    "title": "...",
    "start": "...",
    "end": "...",
    "location": "...",
    "description": "...",
    "all_day": false
  }
]

Dates must be ISO-8601.

## Errors

- Non-zero exit code on failure.
- Human-readable error messages.
- Never produce partial JSON.

## Non-goals

- Calendar editing
- Daemon mode
- Local database
- GUI
- Multiple output formats

## Constraints

- Rust
- Stable toolchain
- Use the Google Calendar REST API directly.
- Avoid the Google SDK.
- Prefer blocking HTTP over async unless necessary.
- Minimize dependencies.

## Acceptance Criteria

Running:

print-cal --days 3 --output events.json

produces a valid JSON file containing the next three days of events.
