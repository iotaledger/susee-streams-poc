# This is inspired by https://blog.logrocket.com/packaging-a-rust-web-service-using-docker/
# cite from the linked article:
#     A good practice when building Docker images to be run in the cloud is to use a multibuild setup
#     where the executable is created in a builder step and then copied into a different, slimmer image.

# First stage for the builder step. Use the official Rust Docker image as the base image
FROM rustlang/rust:nightly AS builder

# Set the working directory to the root of the project
WORKDIR /app

# Copy the entire project to the container
COPY . .

# Build the project with Cargo
RUN cargo build --release

# Create a new stage with a minimal image for the management-console
FROM debian:bullseye-slim as management-console

# Install any necessary system dependencies
# For example, if your Rust project requires OpenSSL, you can install it here
# RUN apt-get update && apt-get install -y libssl-dev

# Copy the built binaries from the previous stage to the current stage
COPY --from=builder /app/target/release/management-console /app/management-console

# Expose the listener port of the message explorer
EXPOSE 8080

# Set the entry point for the container to automatically run the message explorer
WORKDIR /app
CMD ["./management-console", "-r",  "0.0.0.0:8080"]

# Create a new stage with a minimal image for the app-srv-connector-mock
FROM debian:bullseye-slim as app-srv-connector-mock
COPY --from=builder /app/target/release/app-srv-connector-mock /app/app-srv-connector-mock
EXPOSE 50001
WORKDIR /app
CMD ["./app-srv-connector-mock", "-l",  "0.0.0.0:50001", "-b", "http://iota-bridge:50000"]

# Create a new stage with a minimal image for the sensor
FROM debian:bullseye-slim as sensor
COPY --from=builder /app/target/release/sensor /app/sensor
COPY --from=builder app/test/payloads/*.json    /app
WORKDIR /app
CMD ["./sensor", "-f", "meter_reading_1_compact.json", "-b", "http://iota-bridge:50000", "--use-lorawan-rest-api"]

# Create a new stage with a minimal image for the iota-bridge
FROM debian:bullseye-slim as iota-bridge
COPY --from=builder /app/target/release/iota-bridge /app/iota-bridge
EXPOSE 50000
WORKDIR /app
CMD ["./iota-bridge", "-l",  "0.0.0.0:50000"]
