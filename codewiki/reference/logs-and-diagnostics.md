---
description: Where the log files are, how to read more of them, and how to open them.
icon: file-lines
---

# Logs and diagnostics

When something does not start, connect or update, read the logs first.

## The log files

The app, the backend and the MCP bridge each write a log. They are in `%LOCALAPPDATA%\CodeWiki\logs`:

| File          | Written by                       | Holds                                                                                                        |
| ------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `app.log`     | The desktop app                  | What the app did, which graphics adapter it uses, and any error.                                             |
| `backend.log` | The backend (`codewiki serve`)   | Its start, failed, refused and slow requests, each step of a sync with its time and a summary, and its stop. |
| `mcp.log`     | The MCP bridge of a coding agent | What the bridge did.                                                                                         |

A log that grows past 1 MB is moved aside to a file ending in `.old.log`. Other `codewiki` commands print warnings in the terminal only.

{% hint style="warning" %}
The logs name the folders of your projects. Read them before you post them in public.
{% endhint %}

## Open the logs

* Settings → Diagnostics → **Logs** → **Logs folder** → **Open**.
* Ctrl+K, then **Open logs folder**.
* The page **The backend is not reachable.** has an **Open logs folder** button.

Each shows the folder in Explorer.

## Read more

Settings → Diagnostics → **Logs** → **App log level** chooses how much `app.log` keeps:

| Level                             | Keeps                   |
| --------------------------------- | ----------------------- |
| **Errors only**                   | Errors.                 |
| **Warnings and errors**           | Warnings and errors.    |
| **Info: the usual amount**        | The usual amount.       |
| **Debug: every request and call** | Every request and call. |
| **Trace: everything**             | Everything.             |

The change applies at once. It does not change the backend's log.

The environment variable `CODEWIKI_LOG=debug` does the same for any of the programs, and wins over the setting. The setting then says **Set by CODEWIKI\_LOG**. Start the app again after you set it.

`CODEWIKI_LOG_DIR` writes the logs to another folder.

## The data folder

Settings → Diagnostics → **Data** → **Data folder** → **Open** shows the folder that holds the indexes, the list of repositories and the saved settings. It is `%LOCALAPPDATA%\CodeWiki`. A wiki itself is in its repository.

## Report a problem

Open an issue at [github.com/Fly-onlyone/CodeWiki/issues](https://github.com/Fly-onlyone/CodeWiki/issues). Say what you did, what you expected and what happened, and attach `app.log` and `backend.log`.

You can also let CodeWiki send [error reports](error-reports.md).
