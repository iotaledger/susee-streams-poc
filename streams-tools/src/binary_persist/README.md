This is a custom binary persistence implementation.

Before it can be used on an arbitrary
combination of communicating systems (e.g. ESP32 talking to AMD64 architecture) it must be tested
before using it in production.

Following problems are not solved by the current implementation:
* Little endian and big endian conflicts
* Versioning conflicts

## TODO

Replace the code in this module by one of the following libraries
* https://github.com/jamesmunns/postcard
* https://github.com/tokio-rs/prost
* https://users.rust-lang.org/t/comparison-of-way-too-many-rust-asn-1-der-libraries/58683
* https://github.com/Geal/nom