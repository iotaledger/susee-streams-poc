# Susee Streams POC - iota client types
This crate contains duplicated code from iota-client and iota-streams library that is needed to compile the sensor app for
embedded MCUs (e.g. ESP32-C3).

This code duplication is of course only a temporary solution.

As long term solution the iota libraries should provide features that allow to only export
the types that are defined in this library.  