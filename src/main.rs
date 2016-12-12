#![feature(io)]

extern crate vec_map;

use std::fmt;
use std::fmt::{Display, Formatter};

use vec_map::VecMap;

fn main() {
    use std::io::Read;

    test(FakeTodoList::new()).unwrap();

    let stdin = std::io::stdin();
    for key in stdin.lock().chars() {
        match key.unwrap() {
            'q' => break,
            key => println!("Received key: {}; press q to quit", key),
        }
    }
}

fn test<T>(mut tasks: T) -> Result<()>
    where T: TodoList + Display
{
    print!("\n=== Initial ===\n");
    let start = tasks.create(String::from("Start making ado"));
    let rusqlite = tasks.create(String::from("Try rusqlite"));
    let onion = tasks.create(String::from("Implement an onion architecture"));
    let rewrite = tasks.create(String::from("Start a simplified rewrite of ado"));
    tasks.create(String::from("Refine the design of ado"));
    println!("{}", tasks);

    print!("\n=== First attempt ===\n");
    try!(tasks.update(start, Task::finish));
    try!(tasks.update(rusqlite, Task::finish));
    println!("{}", tasks);

    print!("\n=== Restart ===\n");
    try!(tasks.update(rewrite, Task::finish));
    println!("{}", tasks);

    print!("\n=== Clean ===\n");
    try!(tasks.update(onion, Task::abandon));
    println!("{}", tasks);

    print!("\n=== Next steps ===\n");
    try!(tasks.update(rusqlite, Task::create));
    tasks.create(String::from("Make ado interactive"));
    println!("{}", tasks);

    Ok(())
}

struct FakeTodoList {
    tasks: VecMap<Task>,
    next_id: usize,
}

impl FakeTodoList {
    fn new() -> FakeTodoList {
        FakeTodoList {
            tasks: VecMap::new(),
            next_id: 0,
        }
    }
}


struct Task {
    status: Status,
    name: String,
}

enum Status {
    Open,
    Done,
    Wont,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum Error {
    AlreadyOpen,
    AlreadyDone,
    AlreadyWont,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", std::error::Error::description(self))
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::AlreadyOpen => "The task is already.create",
            Error::AlreadyDone => "The task is already finished",
            Error::AlreadyWont => "The task has already been abandoned",
        }
    }
}

trait TodoList {
    type IdType: Copy;

    fn create(&mut self, name: String) -> Self::IdType;

    fn each<F>(&mut self, f: F) where F: FnMut(&mut Task);
    fn update<F, R>(&mut self, id: Self::IdType, f: F) -> R where F: FnOnce(&mut Task) -> R;
}

impl Task {
    fn finish(&mut self) -> Result<()> {
        match self.status {
            Status::Done => Err(Error::AlreadyDone),
            _ => {
                self.status = Status::Done;
                Ok(())
            }
        }
    }

    fn abandon(&mut self) -> Result<()> {
        match self.status {
            Status::Done => Err(Error::AlreadyDone),
            Status::Wont => Err(Error::AlreadyWont),
            _ => {
                self.status = Status::Wont;
                Ok(())
            }
        }
    }

    fn create(&mut self) -> Result<()> {
        match self.status {
            Status::Open => Err(Error::AlreadyOpen),
            _ => {
                self.status = Status::Open;
                Ok(())
            }
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let check = match self.status {
            Status::Open => "[ ]",
            Status::Done => "[x]",
            Status::Wont => " X ",
        };
        write!(f, "{} {}", check, self.name)
    }
}

impl TodoList for FakeTodoList {
    type IdType = usize;

    fn create(&mut self, name: String) -> Self::IdType {
        let id = self.next_id;
        self.next_id += 1;

        self.tasks.insert(id,
                          Task {
                              status: Status::Open,
                              name: name,
                          });

        id
    }

    fn update<F, R>(&mut self, id: Self::IdType, f: F) -> R
        where F: FnOnce(&mut Task) -> R
    {
        f(&mut self.tasks[id])
    }

    fn each<F>(&mut self, mut f: F)
        where F: FnMut(&mut Task)
    {
        for (_, mut task) in self.tasks.iter_mut() {
            f(task)
        }
    }
}

impl Display for FakeTodoList {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let list_format = self.tasks
            .values()
            .filter(|task| match task.status {
                Status::Wont => false,
                _ => true,
            })
            .map(|task| format!("{}", task))
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "{}", list_format)
    }
}
