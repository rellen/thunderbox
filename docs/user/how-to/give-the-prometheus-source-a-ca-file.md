# Give the Prometheus source a CA file

This guide repairs the error `No CA certificates were loaded from the system`. A
CA is a certificate authority, which is an organization that signs the
certificate of a server. The data source `prometheus` reads the certificate of
each CA from the machine when the dashboard loads.

On Linux, the data source returns an error when the machine holds no such
certificate. It does so for an address that starts with `http://` as well. The
program then stops before it opens the dashboard, and it writes this text:

```
Error: The program cannot make the data sources of the dashboard.

Caused by:
    The dashboard is not correct. thunderbox cannot build a client. builder error: unexpected error: No CA certificates were loaded from the system
```

macOS reads the certificates of its own system, so it needs no step of this
guide.

## Install the certificates of your distribution

1. Install the package of CA certificates of your distribution. On Debian and on
   Ubuntu, run these commands:

   ```
   sudo apt-get update
   sudo apt-get install ca-certificates
   ```

2. Run the program with your dashboard file again.

The program now opens the dashboard.

## Name a file of certificates

Use this way when you cannot install a package.

1. Copy a file that holds the certificate of each CA into your home folder. The
   file `/etc/ssl/certs/ca-certificates.crt` of a Debian machine is such a file.
2. Give the path of that file to the program in the environment variable
   `SSL_CERT_FILE`:

   ```
   SSL_CERT_FILE=$HOME/ca-certificates.crt thunderbox my-dashboard.toml
   ```

The program now opens the dashboard.

## The pages for the facts and the reasons

- [The data source prometheus](../reference/data-sources/prometheus.md) states
  the file of certificates on Linux.
- [The command line](../reference/command-line.md) states each error that the
  program writes before the terminal opens.
