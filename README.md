# Reverse Proxy Server with Pingora

This example focuses on how to implement a reverse proxy server using [Pingora](https://github.com/cloudflare/pingora).

## Pingora

> Pingora is a Rust framework to build fast, reliable and programmable networked systems. Pingora is battle tested as it has been serving more than 40 million Internet requests per second for more than a few years.

## Implemented features

- Round-robin load balancing
- Health checking for upstream servers
- Rate limiting
- Integration with Opentelemetry

## How to run

1. Start OpenTelemetry collector and OpenObserve containers

    ```console
    $ docker compose up -d
    ```

2. Run the proxy server

    ```console
    $ cargo run
    ```

3. Make a sample request to the proxy server

    ```console
    $ curl localhost:6188 -H "appid:1" -vi
    ```

## Visualizing signals with OpenObserve

<img width="1502" alt="Image" src="https://github.com/user-attachments/assets/6224ee08-dc8e-488c-9c81-49fcc0ca69dc" />