# httpot
## Description
httpot is an HTTP honeypot, written as excuse to write a HTTP server
in Rust. There are two distinct honeypot modes today:
  * [PHP Easter Eggs](/src/lib/honeypot/php.rs)
  * [Fake Directory Listing](/src/lib/fs/fake.rs)

Both are intended to keep driveby crawlers on my servers busy.
