# ADR 0064: An address of plain HTTP needs no CA file

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 9 |
| Code | `crates/thunderbox-source-prometheus/src/lib.rs`, `default.nix`, `docs/user/reference/data-sources/prometheus.md` |
| Tests | `the_error_of_the_client_names_its_cause` |
| Related | [ADR 0055](0055-build-the-package-with-a-nix-flake.md), [ADR 0056](0056-the-check-phase-of-the-package-reads-a-ca-file.md) |

## Context

A machine on Linux that holds no CA certificate cannot read a Prometheus server
over plain HTTP. A CA is a certificate authority.

`reqwest` 0.13.4 builds the verifier of certificates when it builds a client,
for every scheme. On Linux, the verifier of `rustls-platform-verifier` 0.7.0
returns an error when the machine holds no CA certificate. `Prometheus::new`
therefore returns an error for `http://127.0.0.1:9090` on such a machine.

A container that holds no package of CA certificates is such a machine. The
sandbox of Nix is another one.

## Decision

The project states the need, and it does not remove it:

- The error names its cause.
- `docs/user/reference/data-sources/prometheus.md` states the need.
- `default.nix` gives the sandbox a file of certificates.
  [ADR 0056](0056-the-check-phase-of-the-package-reads-a-ca-file.md) states
  that rule.

## Consequences

A person who reads a server of plain HTTP from such a machine must install the
package of CA certificates. The variable `SSL_CERT_FILE` can also name a file
of such certificates.

## Measurements

A test program ran with `SSL_CERT_FILE` set to a file which does not exist. It
wrote this error:

```
builder error: unexpected error: No CA certificates were loaded from the system
```

The same program built the client with the file of CA certificates of this
machine. Without that file, 22 tests of the workspace fail.

## The question that remains

The work that remains removes the need for a file of certificates from a
client of a plain HTTP address. Two designs could remove the need, and nobody
has examined either one:

- The client can add the CA certificates that the crate `webpki-root-certs`
  holds, through `ClientBuilder::tls_certs_merge`. The verifier then returns no
  error for a machine that holds no CA certificate. The client would also trust
  each CA of that crate. It would then trust a CA that a person removed from
  the machine.
- The client of a plain HTTP address can skip the verifier of the machine.
  `ClientBuilder::tls_backend_preconfigured` takes a configuration of
  `rustls`. That configuration must come from the release of `rustls` that
  `reqwest` uses. This project would then take `rustls` as a dependency of its
  own.

The next design answers one question. Must a client of a plain HTTP address
read any certificate at all?
