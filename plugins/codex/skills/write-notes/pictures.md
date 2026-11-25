# Pictures in a wiki note

A picture is a short program. The CodeWiki app draws it; nothing else reads it. It is sent in `submit_note` as one entry of `pictures`: the `section` it stands in, and its `source`.

## The four components

| Component | Arguments | Draws |
|---|---|---|
| `Picture` | `children` | The picture. `root` must be one. Its children are a `Flow`, `Callout`s, or both. |
| `Flow` | `steps`, `title?` | Boxes joined by arrows, read from the first to the last. |
| `Step` | `title`, `text?` | One box: a title, and a sentence under it. |
| `Callout` | `text`, `tone?`, `title?` | A marked box for a key point or a warning. |

The `tone` of a callout is `info`, `warning`, `success` or `danger`. Without one it is `info`.

## The form of a program

- One statement on a line: `name = Component(arguments)`.
- The first statement is `root = Picture([...])`.
- Text stands in double quotes. A list stands in square brackets.
- An argument marked `?` may be left out. Arguments come in the order of the table.
- A name may be used before the line that defines it.

## Limits

A picture that breaks one is refused, and the answer names the limit.

- A note has at most 4 pictures, and a section at most one.
- A picture has at most one `Flow` and at most 2 `Callout`s.
- A `Flow` has 2 to 7 steps.
- A step's title has at most 40 characters, its text at most 160.
- A callout's text has at most 240 characters, its title at most 40.
- A picture's source has at most 4000 bytes.
- Plain words only: no backticks, no `[[links]]`, no Markdown.

## What makes a picture explain

- Each step is something that happens, in the words a person would use: "You turn the switch on", not "set_enabled(true)".
- The steps go from a cause to its result. A list of parts is not a flow; write that as text.
- A step's text adds one fact the title lacks: where, under which condition, with what result.
- A callout holds the one thing a reader must not miss. Two callouts are the most; with more, none stands out.
- A picture repeats nothing the section already says in a sentence.

## Examples

A flow with a warning, for the section `How it works` of a note about starting an app at sign-in:

```openui
root = Picture([flow, limit])
flow = Flow([on, write, signin, start], "From the switch to a running app")
on = Step("You turn the switch on", "Start with Windows, on the Appearance page of Settings.")
write = Step("One value is written", "The path of the program, under the Run key of your user.")
signin = Step("You sign in to Windows", "Windows reads the Run key and starts what it lists.")
start = Step("The app starts in the tray", "It opens no window until you ask for one.")
limit = Callout("Only an installed copy offers the switch. A copy run from a build folder would be started from a path that may be gone tomorrow.", "warning", "Installed copies only")
```

A callout alone, for the section `Gotchas` of a note about a settings file:

```openui
root = Picture([key])
key = Callout("A key is stored encrypted for your Windows user. Copying the file to another computer or another user gives a file whose keys cannot be read.", "danger", "The file does not travel")
```
