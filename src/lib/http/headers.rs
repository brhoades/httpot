use std::collections::{
    hash_map::{Entry, Iter},
    HashMap,
};

use lazy_static::lazy_static;

/// Headers are key-value with multiple values. Adding a new header
/// does not overwrite existing values, it only appends.
#[derive(Debug, Default, Clone)]
pub struct Headers(HashMap<String, Vec<String>>);

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry(&mut self, key: String) -> Entry<String, Vec<String>> {
        self.0.entry(key)
    }

    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.0.get(key)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Vec<String>> {
        self.0.get_mut(key)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// renders the HTTP request/response format header listing
    /// with newlines.
    pub fn to_string(&self) -> String {
        self.clone().into_string()
    }

    pub fn into_string(self) -> String {
        self.0
            .into_iter()
            .fold(vec![], |mut acc: Vec<String>, (key, values)| {
                if values.len() == 0 {
                    // ???
                } else if values.len() > 1 {
                    match KNOWN_LIST_HEADERS.get(key.to_lowercase().as_str()) {
                        Some(delim) => {
                            acc.push(format!("{}: {}", key, &values.as_slice().join(delim)))
                        }
                        None => acc.extend(values.iter().map(|v| format!("{}: {}", key, v))),
                    }
                } else {
                    acc.push(format!("{}: {}", key, &values.first().unwrap()));
                }

                acc
            })
            .as_slice()
            .join("\n")
    }

    pub fn add<S: ToString>(&mut self, k: &str, v: S) -> &mut Self {
        self.0
            .entry(k.to_string())
            .and_modify(|values| values.push(v.to_string()))
            .or_insert_with(|| vec![v.to_string()]);
        self
    }

    pub fn iter(&self) -> Iter<String, Vec<String>> {
        self.0.iter()
    }
}

lazy_static! {
    // Mapping of known multivalue headers in lowercase
    // to their delimeter.
    pub static ref KNOWN_LIST_HEADERS: HashMap<&'static str, &'static str> = [
        ("a-im", ","),
        ("accept", ","),
        ("accept-charset", ","),
        ("accept-encoding", ","),
        ("accept-language", ","),
        ("access-control-request-headers", ","),
        ("cache-control", ","),
        ("cookie", ";"),
        ("connection", ","),
        ("content-type", ";"),
        ("content-encoding", ","),
        ("expect", ","),
        ("forwarded", ","),
        ("if-match", ","),
        ("if-none-match", ","),
        ("prefer", ";"),
        ("range", ","),
        ("te", ","),
        ("trailer", ","),
        ("transfer-encoding", ","),
        ("upgrade", ","),
        ("via", ","),
        ("warning", ","),
        ("x-forwarded-for", ","),
    ].into_iter().collect::<HashMap<&'static str, &'static str>>();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize_headers() {
        let mut h = Headers::default();
        h.add("Connection", "Close")
            .add("Cookie", "session=123")
            .add("Cookie", "foo=bar");

        let mut count = 0;
        for l in h.into_string().lines() {
            count += 1;
            match l.split_once(": ") {
                Some(("Connection", v)) => assert_eq!("Close", v),
                Some(("Cookie", values)) => {
                    assert_eq!(
                        vec!["session=123", "foo=bar"],
                        values.split(";").collect::<Vec<&str>>()
                    )
                }
                other => panic!("unexpected extra header: {:?}", other),
            }
        }

        assert_eq!(2, count, "expected to read both headers added");
    }
}
