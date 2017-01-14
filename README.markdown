# Ado

A simple todo list application written in Rust.

![Screenshot of Ado](screenshots/cropped.jpg)

## Usage

Ado is based around tasks.
Tasks have a name as well as one of three statuses:
`WONT`, `TODO`, and `DONE`.
Tasks can be created and deleted, and moved between statuses.
The `$PWD/.ado` directory is used for storing state.
Ado does nothing else.

The keybindings are vi-like:

|  Key | Command |
|-----:|:--------|
|  `j` | Select next task |
|  `k` | Select previous task |
|  `h` | Move the current task to previous status |
|  `l` | Move the current task to next status |
|  `o` | Open a new task |
|  `D` | Delete the current task |
| `dd` | Delete the current task |
| `gg` | Select the first task |
|  `G` | Select the last task |

## Motivation

I wrote this for a couple of reasons.

First, there was no preexisting:

- terminal based,
- consistently working, and
- simple
- TODO list application with
- vim bindings.

At least not one I could find.

Second, I wanted to try out a couple of OO-related ideas in
Rust and see how they translated; particularly decorators.
This is why the code is more complex than it needs to be.
