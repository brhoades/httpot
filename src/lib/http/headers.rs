use std::collections::{hash_map::Entry, HashMap};

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
        // safe: don't render multiple headers for multiple values
        // since we don't have / want to encode the list of headers
        // where that's OK.
        self.0
            .into_iter()
            .fold(vec![], |mut acc: Vec<String>, (key, values)| {
                acc.push(key + ": " + &values.as_slice().join(", "));
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize_headers() {
        let mut h = Headers::default();
        h.add("Connection", "Close")
            .add("Set-Cookie", "session=123")
            .add("Set-Cookie", "foo=bar");

        let mut count = 0;
        for l in h.into_string().lines() {
            count += 1;
            match l.split_once(": ") {
                Some(("Connection", v)) => assert_eq!("Close", v),
                Some(("Set-Cookie", values)) => {
                    assert_eq!(
                        vec!["session=123", "foo=bar"],
                        values.split(", ").collect::<Vec<&str>>()
                    )
                }
                other => panic!("unexpected extra header: {:?}", other),
            }
        }

        assert_eq!(2, count, "expected to read both headers added");
    }
}
