---
title: Proxy Configuration
slug: /proxy-configuration
description: Route Mnethos API traffic through an HTTP/HTTPS proxy using the standard HTTP_PROXY, HTTPS_PROXY, and NO_PROXY environment variables.
---

# Proxy Configuration

If you're behind a corporate firewall, a VPN exit node, or any network that requires outbound traffic to go through a proxy, Mnethos respects the standard `HTTP_PROXY` and `HTTPS_PROXY` environment variables. Set them once, and every API call Mnethos makes — to OpenAI, Anthropic, OpenRouter, or any custom provider — will flow through your proxy.

## Setting the Proxy

Mnethos reads two standard environment variables:

| Environment Variable | Protocol | Example Value |
|----------------------|----------|----------------------------------|
| `HTTP_PROXY`         | HTTP     | `http://proxy.company.com:8080`  |
| `HTTPS_PROXY`        | HTTPS    | `http://proxy.company.com:8080`  |
| `NO_PROXY`           | —        | `localhost,127.0.0.1,.internal.io` |

**Both `HTTP_PROXY` and `HTTPS_PROXY` accept an HTTP proxy URL** — even for HTTPS traffic. The connection to the target server is tunneled through the proxy using the `CONNECT` method, so the proxy itself doesn't see the encrypted payload.

There are three ways to set them, depending on how permanent you want the configuration to be.

**`~/.env` — persistent, Mnethos-only**

The `.env` file in your home directory is loaded by Mnethos on every run. This is the right choice when you want the proxy active for Mnethos without affecting other tools on your system:

```bash
# ~/.env
HTTP_PROXY=http://proxy.company.com:8080
HTTPS_PROXY=http://proxy.company.com:8080
NO_PROXY=localhost,127.0.0.1,.internal.company.com
```

**`~/.zshrc` (or `~/.bashrc`) — persistent, system-wide**

Adding the variables to your shell profile makes them available to every process in your terminal, not just Mnethos. Use this when all outbound tools on your machine need to go through the proxy:

```bash
# ~/.zshrc
export HTTP_PROXY=http://proxy.company.com:8080
export HTTPS_PROXY=http://proxy.company.com:8080
export NO_PROXY=localhost,127.0.0.1,.internal.company.com
```

Reload your shell after editing (`source ~/.zshrc`) or open a new terminal.

**Current session — temporary**

To route traffic through a proxy only for the duration of your current terminal session:

```bash
export HTTP_PROXY=http://proxy.company.com:8080
export HTTPS_PROXY=http://proxy.company.com:8080
```

The variables are gone when the session ends.

## Authenticated Proxies

If your proxy requires a username and password, embed the credentials in the URL:

```bash
HTTP_PROXY=http://username:password@proxy.company.com:8080
HTTPS_PROXY=http://username:password@proxy.company.com:8080
```

> **Security Warning**
> Proxy credentials embedded in URLs can appear in shell history, process listings, and log files. Prefer storing them in your `~/.env` file with restricted permissions (`chmod 600 ~/.env`) rather than exporting them directly in your terminal.

## How Traffic Flows

Mnethos makes HTTPS requests to AI provider APIs. When `HTTPS_PROXY` is set, the flow looks like this:

```
Mnethos
   |
   | CONNECT api.openai.com:443
   v
Proxy Server (proxy.company.com:8080)
   |
   | Tunnels encrypted TLS connection
   v
AI Provider API (api.openai.com)
```

The proxy only sees that a tunnel is being opened — the TLS handshake and all request/response content remain encrypted end-to-end between Mnethos and the AI provider.

## Bypassing the Proxy for Specific Hosts

`NO_PROXY` accepts a comma-separated list of hostnames, IP addresses, and domain suffixes that should bypass the proxy:

```bash
# Bypass proxy for localhost, a specific IP, and anything under .internal.company.com
NO_PROXY=localhost,127.0.0.1,192.168.1.0/24,.internal.company.com
```

Leading dots (`.internal.company.com`) match any subdomain of that domain.

## Proxy with Custom Certificates

Corporate proxies commonly perform TLS inspection — they intercept HTTPS connections, decrypt them, inspect the traffic, and re-encrypt using their own certificate authority. If Mnethos fails to connect with certificate errors, your proxy is likely doing this.

The fix is to add your corporate CA certificate to Mnethos's trusted roots:

```bash
# ~/.env
HTTPS_PROXY=http://proxy.company.com:8080
# Trust the corporate CA that signs the proxy's certificates
MNETHOS_HTTP__ROOT_CERT_PATHS=/etc/ssl/certs/corporate-ca.pem
```

Mnethos accepts certificates in PEM, CRT, or CER format. For multiple certificates, provide a comma-separated list of paths.

If you cannot obtain the CA certificate and need to connect urgently in a controlled environment:

```bash
MNETHOS_HTTP__ACCEPT_INVALID_CERTS=true
```

> **Security Warning**
> `MNETHOS_HTTP__ACCEPT_INVALID_CERTS=true` disables all certificate validation. This removes protection against man-in-the-middle attacks. Only use it in isolated development environments where you control the network — never in production or on untrusted networks.
