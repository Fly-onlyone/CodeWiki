---
description: Nothing is sent unless you turn it on. What a report holds.
icon: shield-halved
---

# Error reports and privacy

CodeWiki sends nothing about you or your code unless you ask it to.

## Turn error reports on

Settings → Diagnostics → **Send error reports** is off until you turn it on.

Then a crash or an error of the app, the backend or the agents' bridge goes to the developer through [Sentry](https://sentry.io). Turning it off stops the reports at once. Turning it on takes effect when each program next starts.

A build that carries no error reporting shows the switch locked: **This build has no error reporting**.

## What a report holds

* what failed, and where in the code;
* the versions of CodeWiki and Windows;
* the log lines just before it.

## What it does not hold

* Every file path is replaced by `<path>` before the report leaves.
* No report holds note text, a search query, a key or your computer's name.
* Sentry keeps neither the network address a report came from nor the city and country of that address.

{% hint style="warning" %}
The log lines can name a repository, a note, or the address of a backend on another machine. Those names are not removed.
{% endhint %}

## For developers

A developer can point a build at their own Sentry project with the environment variable `SENTRY_DSN`. The switch then shows as set by that variable.

See also [Logs and diagnostics](logs-and-diagnostics.md).
