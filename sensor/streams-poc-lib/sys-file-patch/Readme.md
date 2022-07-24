This folder contains copies of three unix/sys files that have been instrumented with debug log println calls.

Without this instumentation the processing fails for unknown reason.

=========>    THIS IS A VERY DIRTY HACK    <============

But this hack allows us to use the streams_poc_lib for the next field_test without spending the time that is needed to find out why the processing fails.

The files are usually located here on your build system:

* ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/sys/unix/fs.rs
* ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs
* ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/fs.rs

Just merge the println() calls into your source tre of the rustup nightly toolchain using a merge tool (e.g. Meld or VSCode).