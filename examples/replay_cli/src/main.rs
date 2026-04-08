use std::env;
use std::process;

use of_persist::{RollingStore, StoredEvent};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let request = match parse_request(&args) {
        Ok(request) => request,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", usage());
            process::exit(2);
        }
    };

    if let Err(err) = run_request(request) {
        eprintln!("replay_cli error: {err:?}");
        process::exit(1);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Request {
    ListVenues {
        data_root: String,
    },
    ListSymbols {
        data_root: String,
        venue: String,
    },
    Replay {
        data_root: String,
        venue: String,
        symbol: String,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    },
}

fn parse_request(args: &[String]) -> Result<Request, String> {
    match args.len() {
        0 => Ok(Request::ListVenues {
            data_root: "data".to_string(),
        }),
        1 => Ok(Request::ListVenues {
            data_root: args[0].clone(),
        }),
        2 => Ok(Request::ListSymbols {
            data_root: args[0].clone(),
            venue: args[1].clone(),
        }),
        3 => Ok(Request::Replay {
            data_root: args[0].clone(),
            venue: args[1].clone(),
            symbol: args[2].clone(),
            from_sequence: None,
            to_sequence: None,
        }),
        4 => Ok(Request::Replay {
            data_root: args[0].clone(),
            venue: args[1].clone(),
            symbol: args[2].clone(),
            from_sequence: Some(parse_sequence_arg(&args[3], "from_sequence")?),
            to_sequence: None,
        }),
        5 => {
            let from_sequence = parse_sequence_arg(&args[3], "from_sequence")?;
            let to_sequence = parse_sequence_arg(&args[4], "to_sequence")?;
            if to_sequence < from_sequence {
                return Err("to_sequence must be greater than or equal to from_sequence".to_string());
            }
            Ok(Request::Replay {
                data_root: args[0].clone(),
                venue: args[1].clone(),
                symbol: args[2].clone(),
                from_sequence: Some(from_sequence),
                to_sequence: Some(to_sequence),
            })
        }
        _ => Err("too many arguments supplied".to_string()),
    }
}

fn parse_sequence_arg(raw: &str, name: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .map_err(|_| format!("{name} must be an unsigned integer"))
}

fn run_request(request: Request) -> Result<(), of_persist::PersistError> {
    match request {
        Request::ListVenues { data_root } => {
            let store = RollingStore::new(data_root)?;
            let venues = store.list_venues()?;
            if venues.is_empty() {
                println!("No venues found.");
                return Ok(());
            }
            for venue in venues {
                println!("{venue}");
            }
            Ok(())
        }
        Request::ListSymbols { data_root, venue } => {
            let store = RollingStore::new(data_root)?;
            let symbols = store.list_symbols(&venue)?;
            if symbols.is_empty() {
                println!("No symbols found for venue {venue}.");
                return Ok(());
            }
            for symbol in symbols {
                println!("{symbol}");
            }
            Ok(())
        }
        Request::Replay {
            data_root,
            venue,
            symbol,
            from_sequence,
            to_sequence,
        } => {
            let store = RollingStore::new(data_root)?;
            let streams = store.list_streams(&venue, &symbol)?;
            if streams.is_empty() {
                println!("No persisted streams found for {venue}/{symbol}.");
                return Ok(());
            }

            println!("venue={venue} symbol={symbol} streams={streams:?}");
            let events = store.read_events_in_range(&venue, &symbol, from_sequence, to_sequence)?;
            if events.is_empty() {
                println!("No events matched the requested range.");
                return Ok(());
            }

            for event in &events {
                println!("{}", format_event(event));
            }
            println!("replayed_events={}", events.len());
            Ok(())
        }
    }
}

fn format_event(event: &StoredEvent) -> String {
    match event {
        StoredEvent::Book(book) => format!(
            "BOOK seq={} side={:?} level={} price={} size={} action={:?}",
            book.sequence, book.side, book.level, book.price, book.size, book.action
        ),
        StoredEvent::Trade(trade) => format!(
            "TRADE seq={} price={} size={} aggressor={:?}",
            trade.sequence, trade.price, trade.size, trade.aggressor_side
        ),
    }
}

fn usage() -> &'static str {
    "Usage:
  replay_cli
  replay_cli <data_root>
  replay_cli <data_root> <venue>
  replay_cli <data_root> <venue> <symbol> [from_sequence] [to_sequence]

Behavior:
  no args                     list venues under ./data
  <data_root>                list venues under the provided root
  <data_root> <venue>        list symbols for the provided venue
  <data_root> <venue> <symbol> [range]
                             print discovered streams and replay merged events
                             optional sequence bounds are inclusive"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_listing_request() {
        let request = parse_request(&[]).expect("request");
        assert_eq!(
            request,
            Request::ListVenues {
                data_root: "data".to_string()
            }
        );
    }

    #[test]
    fn parses_symbol_replay_with_range() {
        let args = vec![
            "data".to_string(),
            "CME".to_string(),
            "ESM6".to_string(),
            "10".to_string(),
            "20".to_string(),
        ];
        let request = parse_request(&args).expect("request");
        assert_eq!(
            request,
            Request::Replay {
                data_root: "data".to_string(),
                venue: "CME".to_string(),
                symbol: "ESM6".to_string(),
                from_sequence: Some(10),
                to_sequence: Some(20),
            }
        );
    }

    #[test]
    fn rejects_inverted_sequence_range() {
        let args = vec![
            "data".to_string(),
            "CME".to_string(),
            "ESM6".to_string(),
            "20".to_string(),
            "10".to_string(),
        ];
        let err = parse_request(&args).expect_err("expected error");
        assert!(err.contains("to_sequence"));
    }
}
