# ado

A personal task management system written in Rust.

## Plans

### Persistence

Persistence will be achieved by flat files.

The system will use a `./.ado/` as a repository.

Each file in this directory will be a task.

File names will be task ids.

File contents be a Task serialised as TOML.
