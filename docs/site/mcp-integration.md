---
title: .mcp.json
slug: /mcp-integration
description: Connect Mnethos agents to external tools, APIs, and services via the Model Context Protocol (MCP).
---

# .mcp.json

MCP lets Mnethos connect agents to external tools, APIs, and services.

## What MCP gives you

With MCP, your agents can:

- Call external APIs and web services
- Use specialized tools from local or remote servers
- Automate browser workflows
- Connect to internal services and data systems

## Quick start

Start with one command, confirm it loaded, then use the tools.

```bash
mnethos mcp import '{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    }
  }
}'
mnethos mcp list
```

## CLI command reference

### `mnethos mcp import`

Import one or more MCP servers from a JSON string.

**Usage**

```bash
mnethos mcp import [OPTIONS] '<json_configuration>'
```

**Options**

- `-s, --scope <SCOPE>`: `local` or `user` (default: `local`)
- `--porcelain`: machine-readable output

**Examples**

Add multiple servers to local scope:

```bash
mnethos mcp import '{
  "mcpServers": {
    "context7": {
      "url": "https://mcp.context7.com/sse"
    },
    "deepwiki": {
      "url": "https://mcp.deepwiki.com/sse"
    },
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    }
  }
}'
```

Add a server to user scope:

```bash
mnethos mcp import --scope user '{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    }
  }
}'
```

Typical output:

```
⏺ Added MCP server 'context7'
⏺ Added MCP server 'deepwiki'
⏺ Added MCP server 'playwright'
```

### `mnethos mcp list`

List configured MCP servers.

**Usage**

```bash
mnethos mcp list
```

**Options**

- `--porcelain`: machine-readable output

### `mnethos mcp show`

Show full configuration for one server.

**Usage**

```bash
mnethos mcp show <server_name>
```

**Options**

- `--porcelain`: machine-readable output

Shows command or URL, arguments, environment variables, and final resolved config.

### `mnethos mcp remove`

Remove one MCP server from a selected scope.

**Usage**

```bash
mnethos mcp remove [OPTIONS] <server_name>
```

**Options**

- `-s, --scope <SCOPE>`: `local` or `user` (default: `local`)
- `--porcelain`: machine-readable output

**Examples**

```bash
# Remove from local project config
mnethos mcp remove playwright
# Remove from user config
mnethos mcp remove --scope user playwright
```

### `mnethos mcp reload`

Reload MCP servers after configuration changes.

**Usage**

```bash
mnethos mcp reload
```

**Options**

- `--porcelain`: machine-readable output

Use this after editing `.mcp.json` manually.

## Manual configuration

If you prefer direct file editing, create or update `.mcp.json`.

```json
{
  "mcpServers": {
    "browser_automation": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-browser"],
      "env": {
        "BROWSER_EXECUTABLE": "/usr/bin/chromium-browser"
      }
    },
    "api_service": {
      "command": "python",
      "args": ["-m", "mcp_server", "--port", "3001"],
      "env": {
        "API_KEY": "your_api_key_here",
        "DEBUG": "true"
      }
    },
    "webhook_server": {
      "url": "http://localhost:3000/events"
    }
  }
}
```

### Server configuration types

#### Command-based server

```json
{
  "server_name": {
    "command": "command_to_execute",
    "args": ["arg1", "arg2", "arg3"],
    "env": {
      "ENV_VAR": "value",
      "ANOTHER_VAR": "another_value"
    }
  }
}
```

#### URL-based server

```json
{
  "server_name": {
    "url": "http://localhost:3000/events"
  }
}
```

### Scope and precedence

MCP configuration can exist in two places:

1. **Local scope**: `.mcp.json` in the current project
2. **User scope**: global Mnethos config directory

Local scope wins over user scope when both define the same server.

> **Note**
> Find your resolved configuration path by running `:info` in the Mnethos Shell.

### Disable a server without deleting it

Set `"disable": true` on a server entry.

```json
{
  "mcpServers": {
    "github": {
      "url": "https://api.githubcopilot.com/mcp/",
      "disable": true
    },
    "weather": {
      "command": "node",
      "args": ["weather-server.js"],
      "disable": false
    }
  }
}
```

Behavior:

- `"disable": true`: server is ignored and not loaded
- `"disable": false` or omitted: server loads normally

## How tools become available to agents

After you add a server, tool registration is automatic.

```
Add MCP server -> Mnethos loads config -> Tools are registered -> All agents can use them
```

You do not need per-agent setup.

To verify which MCP tools are available to your current agent, run:

```
:tools
```

Use this whenever you switch agents and want to confirm the active tool list.

## Example setups

### Browser automation

```json
{
  "mcpServers": {
    "browser": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-browser"],
      "env": {
        "HEADLESS": "false",
        "VIEWPORT_WIDTH": "1920",
        "VIEWPORT_HEIGHT": "1080"
      }
    }
  }
}
```

Use this for UI testing, data extraction, and scripted page interactions.

### External API integration

```json
{
  "mcpServers": {
    "weather_api": {
      "command": "python",
      "args": ["-m", "weather_mcp_server"],
      "env": {
        "WEATHER_API_KEY": "your_api_key",
        "DEFAULT_LOCATION": "San Francisco"
      }
    }
  }
}
```

Use this for real-time data access and API-backed workflows.

### Development tool integration

```json
{
  "mcpServers": {
    "database_tools": {
      "command": "node",
      "args": ["database-mcp-server.js"],
      "env": {
        "DB_CONNECTION_STRING": "postgresql://user:pass@localhost:5432/db",
        "QUERY_TIMEOUT": "30000"
      }
    }
  }
}
```

Use this for database operations, schema work, and migration tooling.

## Security checklist

- Store secrets in environment variables, not inline config
- Grant minimum server permissions
- Prefer HTTPS for URL-based servers
- Rotate API keys and access tokens regularly

## Troubleshooting

### Server connection failures

- Verify server URL and port
- Check network reachability
- Confirm required environment variables
- Validate credentials and tokens

### Command execution failures

- Verify command path and arguments
- Check runtime dependencies
- Confirm file permissions
- Re-check environment variables

### Configuration issues

- Validate `.mcp.json` syntax
- Confirm local vs user scope expectations
- Check whether the server is disabled
- Run `mnethos mcp list` to confirm loaded servers

## What to do next

Add one server you need today, verify it with `mnethos mcp list`, and use it in your next agent session. That gives you the fastest path to a real MCP workflow.
