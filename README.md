# Reverse Proxy Server with Pingora

This example focuses on how to implement a reverse proxy server using [Pingora](https://github.com/cloudflare/pingora).

## Pingora

> Pingora is a Rust framework to build fast, reliable and programmable networked systems. Pingora is battle tested as it has been serving more than 40 million Internet requests per second for more than a few years.

## Implemented features

- Round-robin load balancing
- Health checking for upstream servers
- Rate limiting
- Integration with Opentelemetry
