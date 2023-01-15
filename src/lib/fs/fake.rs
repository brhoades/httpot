use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::{
    distributions::{Alphanumeric, DistString},
    prelude::*,
    Fill,
};
use typed_html::{dom::DOMTree, html, text, types::Metadata};

use crate::http::response::{Response, ResponseBuilder};

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
        .map(|_| Node::Folder(String::default()))
        .chain((0..files).map(|_| Node::File(String::default())))
        .map(|mut n| {
            rng.fill(&mut n);
            n
        })
        .collect()
}

pub fn gen_fake_listing<T: Hash>(seed: T, path: &str) -> Response<Vec<u8>> {
    let nodes = gen_fake_nodes(seed, path);

    let doc: DOMTree<String> = html!(
        <html>
          <head>
            <title>{ text!("Directory Listing: {}", path) }</title>
            <meta name=Metadata::Description content="Generated Directory Listing"/>
          </head>
          <body>
            <div>
              <ul>
                { nodes.into_iter().map(|n| {
                    let fullpath = path.to_owned() + &"/".to_owned() + n.name();
                    html!(
                      <li>
                        <a href=fullpath>{ text!("{}", n.name()) }</a>
                      </li>
                    )
                  })
                }
              </ul>
            </div>
          </body>
        </html>
    );

    let b = doc.to_string();
    let b = b.as_bytes();
    ResponseBuilder::default()
        .add_header("Content-Type", "text/html")
        .add_header("Content-Length", b.len())
        .status_code(200)
        .body(b)
        .build()
        .unwrap()
}

#[derive(Debug, Clone)]
enum Node {
    File(String),
    Folder(String),
}

impl Node {
    pub fn name(&self) -> &String {
        match self {
            Node::File(f) => f,
            Node::Folder(n) => n,
        }
    }
}

impl Fill for Node {
    fn try_fill<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), rand::Error> {
        use Node::*;

        match self {
            File(ref mut f) => *f = string_of_size(rng, 4, 10) + "." + &string_of_size(rng, 1, 3),

            Folder(ref mut n) => *n = string_of_size(rng, 6, 15),
        }

        Ok(())
    }
}

fn string_of_size<R: Rng + ?Sized>(rng: &mut R, min: usize, max: usize) -> String {
    let size = rng.gen_range(min..=max);
    Alphanumeric.sample_string(rng, size)
}
