#![feature(io)]

extern crate vec_map;

use std::fmt;
use std::fmt::{Display, Formatter};

use vec_map::VecMap;

fn main() {
    use std::io::Read;

    let mut todo_list = FakeTodoList::new();
    test(&mut todo_list).unwrap();
    let mut task_picker: TaskPicker<FakeTodoList> = TaskPicker {
        position: 0,
        tasks: todo_list,
    };

    let mut todo_list = FakeTodoList::new();
    todo_list.create_finished("Start making ado");
    todo_list.create_finished("Try rusqlite");
    todo_list.create_abandoned("Implement a onion architecture");
    todo_list.create_finished("Start simplified rewrite of ado");
    todo_list.create("Refine the design of ado");
    todo_list.create("Make ado interactive");
    todo_list.create("Implement new task creation");
    todo_list.create("Implement persistence");
    todo_list.create("Have ado use unbuffered input");
    todo_list.create("Eliminate the 'history' e.g. by redrawing the screen");

    let stdin = std::io::stdin();
    for key in stdin.lock().chars() {
        match key.unwrap() {
            'q' => break,
            '\n' => println!("{}\n press q to quit", task_picker),
            'j' => task_picker.down(),
            'k' => task_picker.up(),
            _ => continue,
        }
    }
}

struct TaskPicker<T> {
    position: usize,
    tasks: T,
}

impl<T, I> TaskPicker<T>
where T: TodoList<IdType = I>,
      I: Copy + From<usize>,
      usize: From<I>,
{
    fn down(&mut self) {
        self.position = match self.tasks.next_id(I::from(self.position)) {
            None => self.position,
            Some(i) => usize::from(i),
        }
    }

    fn up(&mut self) {
        self.position = match self.tasks.next_back_id(I::from(self.position)) {
            None => self.position,
            Some(i) => usize::from(i),
        }
    }
}

impl<T> Display for TaskPicker<T>
where T: Display + TodoList
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut strings = Vec::new();
        self.tasks.enumerate(|id, task| {
            let marker = if id == self.position { ">" } else { " " };
            strings.push(format!("{} {}", marker, task));
        });
        write!(f, "{}", strings.join("\n"))
    }
}

fn test<T>(tasks: &mut T) -> Result<()>
    where T: TodoList + Display
{
    print!("\n=== Initial ===\n");
    let start = tasks.create("Start making ado");
    let rusqlite = tasks.create("Try rusqlite");
    let onion = tasks.create("Implement an onion architecture");
    let rewrite = tasks.create("Start a simplified rewrite of ado");
    tasks.create("Refine the design of ado");
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
    tasks.create("Make ado interactive");
    tasks.create("Implement new task creation");
    tasks.create("Implement persistence");
    tasks.create("Have ado use unbuffered input");
    tasks.create("Eliminate the 'history' e.g. by redrawing the screen");
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

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Status::Open => write!(f, "[ ]"),
            Status::Done => write!(f, "[x]"),
            Status::Wont => write!(f, " X "),
        }
    }
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

    fn create(&mut self, name: &str) -> Self::IdType;

    fn each<F>(&mut self, f: F) where F: FnMut(&mut Task);
    fn update<F, R>(&mut self, id: Self::IdType, f: F) -> R where F: FnOnce(&mut Task) -> R;

    fn enumerate<F>(&self, f: F) where F: FnMut(usize, &Task);

    fn contains_key(&self, id: Self::IdType) -> bool;
    fn next_id(&self, id: Self::IdType) -> Option<Self::IdType>;
    fn next_back_id(&self, id: Self::IdType) -> Option<Self::IdType>;

    fn create_finished(&mut self, name: &str) -> Result<Self::IdType> {
        let id = self.create(name);
        try!(self.update(id, Task::finish));
        Ok(id)
    }

    fn create_abandoned(&mut self, name: &str) -> Result<Self::IdType> {
        let id = self.create(name);
        try!(self.update(id, Task::abandon));
        Ok(id)
    }
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

    fn create(&mut self, name: &str) -> Self::IdType {
        let id = self.next_id;
        self.next_id += 1;

        self.tasks.insert(id,
                          Task {
                              status: Status::Open,
                              name: String::from(name),
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
            f(task);
        }
    }

    fn enumerate<F>(&self, mut f: F)
        where F: FnMut(usize, &Task)
    {
        for (id, mut task) in self.tasks.iter() {
            f(id, task);
        }
    }

    fn contains_key(&self, id: Self::IdType) -> bool {
        self.tasks.contains_key(id)
    }

    fn next_id(&self, id: Self::IdType) -> Option<Self::IdType> {
        if id < self.next_id - 1 {
            Some(id + 1)
        } else {
            None
        }
    }

    fn next_back_id(&self, id: Self::IdType) -> Option<Self::IdType> {
        if id > 0 {
            Some(id - 1)
        } else {
            None
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
