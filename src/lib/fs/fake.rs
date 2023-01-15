use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use chrono::{offset::Utc, DateTime, Datelike, TimeZone, Timelike};
use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
    Fill,
};
use typed_html::types::Datetime;
use typed_html::{dom::DOMTree, html, text, types::Metadata};

use crate::{
    http::response::{Response, ResponseBuilder},
    prelude::*,
};

// hashes path and seed together
fn hash_path_seed<T: Hash>(seed: T, path: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    path.hash(&mut hasher);
    hasher.finish()
}

/// Return a rendered listing links provided with the same named
/// subpath. The seed is used with the provided path to deterministically
/// generate random directories and folders.
fn gen_fake_nodes<T: Hash>(seed: T, path: &str) -> Vec<Node> {
    let mut rng = StdRng::seed_from_u64(hash_path_seed(seed, path));

    let files = rng.gen_range(2..=8);
    let folders = rng.gen_range(4..=15);

    (0..folders)
        .map(|_| Node::Right(Default::default()))
        .chain((0..files).map(|_| Node::Left(Default::default())))
        .map(|mut n| {
            rng.fill(&mut n);
            n
        })
        .collect()
}

pub fn gen_fake_listing<T: Hash>(seed: T, path: &str) -> Response {
    let nodes = gen_fake_nodes(seed, path);
    let basepath = if path == "" {
        "/".to_string()
    } else if path.ends_with("/") {
        path.to_string()
    } else {
        path.to_owned() + "/"
    };

    let doc: DOMTree<String> = html!(
        <html>
          <head>
            <title>{ text!("Index of {}", basepath) }</title>
            <meta name=Metadata::Description content="Generated Directory Listing"/>
          </head>
          <body>

            <h1>{ text!("Index of {}", basepath) }</h1>
            <hr />
            <pre>
              <a href="../">"../"</a> "\n"
              { nodes.into_iter().map(|n| {
                  html!(
                    <span>
                      <a href=n.name()>{ text!("{}", n.name()) }</a>
                      "          "
                      "\n"
                    </span>
                  )
                })
              }
            </pre>
            <hr />
          </body>
        </html>
    );

    let doc = doc.to_string();
    ResponseBuilder::ok()
        .add_header("Content-Type", "text/html")
        .add_header("Content-Length", doc.len())
        .body(doc)
        .build()
        .unwrap()
}

type Node = Either<File, Folder>;

#[derive(Debug)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

#[derive(Debug, Clone, Default)]
struct File {
    name: String,
    modified_at: DateTime<Utc>,
    size: usize,
}

#[derive(Debug, Clone, Default)]
struct Folder {
    name: String,
    modified_at: DateTime<Utc>,
}

impl Node {
    pub fn name(&self) -> String {
        match self {
            Node::Left(n) => n.name.to_owned(),
            Node::Right(n) => n.name.to_owned() + "/",
        }
    }
}

impl Fill for Node {
    fn try_fill<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
        match self {
            Node::Left(ref mut f) => {
                f.name = string_of_size(rng, 4, 10) + "." + &string_of_size(rng, 1, 3);
                f.modified_at = plausible_datetime(rng).unwrap_or_default();
                f.size = rng.gen_range(0..=(32 * 1024 * 1024));
            }
            Node::Right(ref mut n) => {
                n.name = string_of_size(rng, 6, 15);
                n.modified_at = plausible_datetime(rng).unwrap_or_default();
            }
        }

        Ok(())
    }
}

fn string_of_size<R: Rng + ?Sized>(rng: &mut R, min: usize, max: usize) -> String {
    let size = rng.gen_range(min..=max);
    Alphanumeric.sample_string(rng, size)
}

/// creates a plausible datetime from 2000 to now which does not go into the future.
fn plausible_datetime<R: Rng + ?Sized>(rng: &mut R) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let year = rng.gen_range(2000..=now.year());
    let month = if year == now.year() {
        rng.gen_range(1..=now.month())
    } else {
        rng.gen_range(1..=12)
    };
    let day = if year == now.year() && month == now.month() {
        rng.gen_range(1..=now.day())
    } else {
        rng.gen_range(1..=get_days_in_month(year, month)?)
    };
    let hour = if year == now.year() && month == now.month() && day == now.day() {
        rng.gen_range(0..=now.hour())
    } else {
        rng.gen_range(0..24)
    };
    let minute =
        if year == now.year() && month == now.month() && day == now.day() && hour == now.hour() {
            rng.gen_range(0..=now.minute())
        } else {
            rng.gen_range(0..60)
        };
    let second = if year == now.year()
        && month == now.month()
        && day == now.day()
        && hour == now.hour()
        && minute == now.minute()
    {
        rng.gen_range(0..=now.second())
    } else {
        rng.gen_range(0..60)
    };

    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .latest()
        .ok_or_else(|| anyhow!("failed to generate random date"))
}

// tyty https://stackoverflow.com/a/58188385
fn get_days_in_month(year: i32, month: u32) -> Result<u32> {
    chrono::NaiveDate::from_ymd_opt(
        match month {
            12 => year + 1,
            _ => year,
        },
        match month {
            12 => 1,
            _ => month + 1,
        },
        1,
    )
    .ok_or_else(|| {
        anyhow!(
            "failed to convert yy/mm {}/{} to get days in month",
            year,
            month
        )
    })?
    .signed_duration_since(
        chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| anyhow!("to create signed duration since {}/{}/{}", year, month, 1))?,
    )
    .num_days()
    .try_into()
    .map_err(|e| anyhow!("failed to subtract for yy/mm {}/{}: {}", year, month, e))
}
