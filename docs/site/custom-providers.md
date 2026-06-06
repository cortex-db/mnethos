---
title: Custom Providers
slug: /custom-providers
description: Configure custom, self-hosted, or gateway LLM providers for Mnethos using provider.json.
---

# Custom Providers

Mnethos ships with built-in support for OpenAI, Anthropic, OpenRouter, Google Vertex AI, DeepSeek, GitHub Copilot, Amazon Bedrock, and many more. For everything else — self-hosted models, enterprise API gateways, regional endpoints, or any service that speaks a supported wire protocol — you add a **custom provider**.

Mnethos merges provider definitions from three sources, in order, with later sources overriding earlier ones field-by-field when the `id` matches:

1. The **built-in** provider catalog embedded in the binary.
2. A **`provider.json`** file at your config base path (`~/.mnethos/provider.json`). This is the primary, fully-featured way to add or override providers and is the format documented here.
3. Inline `providers` entries in [`.mnethos.toml`](/docs/mnethos-toml/) (a convenience subset — see [Inline alternative](#inline-mnethostoml-alternative) below).

For built-in providers, the easiest way to add credentials and switch is `mnethos provider login` and the `:provider` command inside a session — you only need `provider.json` to add a **new** provider or change a built-in's endpoint.

## Adding a Provider

Create `~/.mnethos/provider.json`. It is a JSON **array** of provider objects. The only required fields are `id` and `url`:

```json
[
  {
    "id": "my-provider",
    "url": "https://my-llm-gateway.internal/v1/chat/completions",
    "api_key_vars": "MY_PROVIDER_API_KEY",
    "response_type": "OpenAI",
    "auth_methods": ["api_key"]
  }
]
```

- `id` is the name you'll reference with the `:provider` command and in the `[session]` block of `.mnethos.toml`.
- `url` is the full chat completions endpoint.
- `api_key_vars` names the environment variable that holds the API key.

## Pointing Sessions at a Provider

The `[session]` block in [`.mnethos.toml`](/docs/mnethos-toml/) sets the default provider and model for every conversation. Set `provider_id` to the `id` you defined:

```toml
[session]
provider_id = "my-provider"
model_id    = "meta-llama/Llama-3.3-70B-Instruct"
```

You can override this per-session from inside Mnethos with the `:provider` command, which lets you switch providers interactively without editing any files.

## Full Provider Field Reference

| Field            | Required | Description                                                                                                                  |
| ---------------- | -------- | -------------------------------------------------------------------------------------------------------------------------- |
| `id`             | Yes      | Unique provider identifier (e.g. `"my_provider"`).                                                                          |
| `url`            | Yes      | Chat completions URL; may contain `{{VAR}}` placeholders substituted from `url_param_vars`.                                 |
| `api_key_vars`   | No       | Name of the environment variable holding the API key for this provider.                                                     |
| `auth_methods`   | No       | Authentication methods; defaults to `["api_key"]`. Use `["google_adc"]` for Google Application Default Credentials, or an OAuth object (see [OAuth providers](#oauth-providers)). |
| `custom_headers` | No       | Object of additional HTTP headers sent with every request to this provider.                                                 |
| `models`         | No       | Model source: a URL string for fetching the model list (may contain `{{VAR}}` placeholders), or an inline array of model objects. |
| `provider_type`  | No       | Provider category: `"llm"` (default), `"context_engine"`, or `"memory"`.                                                    |
| `response_type`  | No       | Wire protocol: `"OpenAI"`, `"OpenAIResponses"`, `"Anthropic"`, `"Bedrock"`, `"Google"`, or `"OpenCode"`.                    |
| `url_param_vars` | No       | List of environment variable names (or option objects) substituted into `{{VAR}}` placeholders in `url` and `models`.        |

## Multiple Custom Providers

Add as many objects to the array as you need:

```json
[
  {
    "id": "local",
    "url": "http://localhost:11434/v1/chat/completions",
    "models": "http://localhost:11434/v1/models",
    "response_type": "OpenAI",
    "auth_methods": ["api_key"]
  },
  {
    "id": "staging-gateway",
    "url": "https://staging-llm.internal/v1/chat/completions",
    "api_key_vars": "STAGING_LLM_KEY",
    "response_type": "OpenAI",
    "auth_methods": ["api_key"]
  }
]
```

Switch between them with `:provider` at any time, or point specific operations (`session`, `commit`, `suggest`) at different entries.

## Overriding a Built-In Provider

If your `id` matches a built-in provider (e.g. `"openai"`, `"anthropic"`), the entry **overrides** that provider's fields rather than creating a new one. This lets you swap the endpoint of a built-in without fully replacing it:

```json
[
  {
    "id": "openai",
    "url": "https://openai-proxy.corp.internal/v1/chat/completions",
    "api_key_vars": "CORP_OPENAI_KEY",
    "response_type": "OpenAI",
    "auth_methods": ["api_key"]
  }
]
```

Entries with a new `id` are appended and become available for model selection alongside the built-ins.

## Environment Variables

Both `api_key_vars` and `url_param_vars` reference environment variable **names** — Mnethos reads the values from your environment at runtime. You can set them in your shell profile or in a `~/.env` file, which Mnethos loads automatically on every run:

```bash
# ~/.env
MY_PROVIDER_API_KEY=sk-...
OPENAI_URL=https://my-llm-gateway.internal/v1
```

## URL Template Variables

Both `url` and `models` support `{{VAR}}` placeholders. Declare the variables to substitute in `url_param_vars`. The simplest form is a plain list of environment variable names:

```json
[
  {
    "id": "openai_compatible",
    "api_key_vars": "OPENAI_API_KEY",
    "url_param_vars": ["OPENAI_URL"],
    "response_type": "OpenAI",
    "url": "{{OPENAI_URL}}/chat/completions",
    "models": "{{OPENAI_URL}}/models",
    "auth_methods": ["api_key"]
  }
]
```

At runtime Mnethos reads each variable named in `url_param_vars` and substitutes its value into the matching `{{VAR}}` placeholder. If a provider has no dynamic URL segments, omit `url_param_vars` or pass an empty list `[]`.

### Constrained and optional parameters

A `url_param_vars` entry can also be an object to constrain the value to a set of options (rendered as a dropdown during setup) or mark it optional. The `url` can use Handlebars conditionals to handle optional segments:

```json
[
  {
    "id": "self_hosted",
    "api_key_vars": "SELF_HOSTED_API_KEY",
    "url_param_vars": [
      { "name": "SSL_SCHEME", "options": ["http", "https"] },
      { "name": "HOST" },
      { "name": "PORT", "optional": true }
    ],
    "response_type": "OpenAI",
    "url": "{{SSL_SCHEME}}://{{HOST}}{{#if PORT}}:{{PORT}}{{/if}}/v1/chat/completions",
    "models": "{{SSL_SCHEME}}://{{HOST}}{{#if PORT}}:{{PORT}}{{/if}}/v1/models",
    "auth_methods": ["api_key"]
  }
]
```

## Static Model List

Instead of a URL, `models` can be an inline array of model objects. This is useful when the provider doesn't expose a model-listing endpoint, or when you want to pin the exact set of models available:

```json
[
  {
    "id": "openai",
    "api_key_vars": "OPENAI_API_KEY",
    "response_type": "OpenAI",
    "url": "https://api.openai.com/v1/chat/completions",
    "auth_methods": ["api_key"],
    "models": [
      {
        "id": "o1",
        "name": "O1",
        "description": "OpenAI's reasoning model with advanced problem-solving capabilities",
        "context_length": 200000,
        "tools_supported": true,
        "supports_parallel_tool_calls": true,
        "supports_reasoning": true,
        "input_modalities": ["text"]
      }
    ]
  }
]
```

Each model object supports the following fields:

| Field                          | Description                                                            |
| ------------------------------ | --------------------------------------------------------------------- |
| `id`                           | Model identifier used in API requests.                                |
| `name`                         | Human-readable display name.                                          |
| `description`                  | Short description of the model.                                       |
| `context_length`               | Maximum context window size in tokens.                                |
| `tools_supported`              | Whether the model supports tool/function calling.                     |
| `supports_parallel_tool_calls` | Whether the model can execute multiple tool calls in parallel.        |
| `supports_reasoning`           | Whether the model supports extended reasoning / chain-of-thought.     |
| `input_modalities`             | List of supported input types, e.g. `["text"]` or `["text", "image"]`. |

## Custom Headers

To send additional headers with every request — for example, to pass a gateway token or routing key — add a `custom_headers` object:

```json
[
  {
    "id": "kimi_coding",
    "api_key_vars": "KIMI_API_KEY",
    "response_type": "OpenAI",
    "url": "https://api.kimi.com/coding/v1/chat/completions",
    "models": "https://api.kimi.com/coding/v1/models",
    "auth_methods": ["api_key"],
    "custom_headers": {
      "User-Agent": "KimiCLI/1.0.0"
    }
  }
]
```

## Google Application Default Credentials

For providers that use Google ADC instead of an API key, set `auth_methods` to `["google_adc"]`:

```json
[
  {
    "id": "vertex-custom",
    "url": "https://us-central1-aiplatform.googleapis.com/v1/projects/my-project/locations/us-central1/endpoints/openapi/chat/completions",
    "response_type": "Google",
    "auth_methods": ["google_adc"]
  }
]
```

## OAuth Providers

`provider.json` also supports OAuth device and authorization-code flows by giving `auth_methods` an object instead of a string. For example, an OAuth device flow:

```json
[
  {
    "id": "my_oauth_provider",
    "response_type": "OpenAI",
    "url": "https://api.example.com/v1/chat/completions",
    "models": "https://api.example.com/v1/models",
    "auth_methods": [
      {
        "oauth_device": {
          "auth_url": "https://example.com/login/device/code",
          "token_url": "https://example.com/login/oauth/access_token",
          "client_id": "your-client-id",
          "scopes": ["read:user"],
          "use_pkce": false
        }
      }
    ]
  }
]
```

OAuth-based providers must be configured in `provider.json` — they are not available in the inline `.mnethos.toml` form.

## Provider with Custom Certificate Authority

If your endpoint sits behind a corporate proxy or uses a private CA, point Mnethos at the certificate via the `[http]` section of `.mnethos.toml`:

```toml
[http]
root_cert_paths = ["/etc/ssl/certs/corp-ca.pem"]
```

Then define the provider in `provider.json` as usual. See [Proxy Configuration](/docs/proxy-configuration/) for the full certificate and proxy setup.

## Inline `.mnethos.toml` Alternative

For quick, simple providers you can define them inline in [`.mnethos.toml`](/docs/mnethos-toml/) under `[[providers]]` instead of a separate `provider.json` file. This form covers `api_key` and `google_adc` auth only (no OAuth) and uses **slightly different field names** than `provider.json`:

- `api_key_var` is **singular** here (in `provider.json` it is `api_key_vars`, plural).
- `url_param_vars` entries are tables with a `name` key (not bare strings).

```toml
[[providers]]
id            = "my-provider"
url           = "https://my-llm-gateway.internal/v1/chat/completions"
api_key_var   = "MY_PROVIDER_API_KEY"
response_type = "OpenAI"
auth_methods  = ["api_key"]

# URL template variables use a table per entry:
[[providers.url_param_vars]]
name = "OPENAI_URL"

# Inline static models use nested tables:
[[providers.models]]
id             = "o1"
name           = "O1"
context_length = 200000
tools_supported = true
```

Both forms are merged into the same provider list; use whichever is more convenient. `provider.json` is recommended when you need OAuth, want to copy a built-in entry verbatim, or prefer to keep providers separate from the rest of your config.

## Verifying the Configuration

Switch to your provider with the `:provider` command inside any Mnethos session to confirm it loads and responds. If Mnethos can't reach the endpoint, it surfaces a connection error — check that `url` is reachable and the API key environment variable is set.

The full list of configuration options for `.mnethos.toml` is documented in [`.mnethos.toml`](/docs/mnethos-toml/).
