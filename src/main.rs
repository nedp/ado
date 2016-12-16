extern crate ncurses;
extern crate vec_map;

use std::fmt;
use std::fmt::{Display, Formatter};

use std::io::prelude::*;
use std::fs::File;

use vec_map::VecMap;

const PATH: &'static str = "./.ado/";

fn main() {
    let mut todo_list = FileTodoList::new();
    // todo_list.create_finished("Start making ado").unwrap();
    // todo_list.create_finished("Try rusqlite").unwrap();
    // todo_list.create_closed("Implement a onion architecture").unwrap();
    // todo_list.create_finished("Start simplified rewrite of ado").unwrap();
    // todo_list.create("Refine the design of ado").unwrap();
    // todo_list.create_finished("Make ado interactive").unwrap();
    // todo_list.create_finished("Implement new task creation").unwrap();
    // todo_list.create("Implement persistence").unwrap();
    // todo_list.create_finished("Have ado use unbuffered input").unwrap();
    // todo_list.create_finished("Eliminate the stdout history").unwrap();
    // todo_list.create_finished("Implement task status toggling (done/open)").unwrap();
    // todo_list.create_finished("Implement task status toggling (open/closed)").unwrap();
    // todo_list.create_finished("Hide the cursor").unwrap();
    // todo_list.create("Create a help screen").unwrap();
    // todo_list.create("Refactor next and next_back to a list of ids").unwrap();
    let mut task_picker = TaskPicker {
        position: todo_list.next_id(0).ok().or(Some(0)).unwrap(),
        tasks: todo_list,
    };

    gui(&mut task_picker).unwrap();
}

fn gui<T>(task_picker: &mut TaskPicker<T>) -> Result<(), Error>
    where T: TodoList<Id = usize>,
{
    use std::error::Error;

    ::ncurses::initscr();
    ::ncurses::noecho();
    ::ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    ::ncurses::noraw();
    ::ncurses::cbreak();

    ::ncurses::printw(&format!("{}", task_picker));
    ::ncurses::refresh();

    loop {
        let result = match ::ncurses::getch() {
            x => {
                match char::from(x as u8) {
                    'q' => break,

                    'h' => task_picker.left(),
                    'j' => task_picker.down(),
                    'k' => task_picker.up(),
                    'l' => task_picker.right(),

                    'o' => {
                        ::ncurses::printw("\nEnter a task summary:\n");
                        ::ncurses::echo();
                        ::ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_VISIBLE);
                        ::ncurses::nocbreak();
                        let mut name = String::new();
                        ::ncurses::getstr(&mut name);
                        ::ncurses::noecho();
                        ::ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
                        ::ncurses::cbreak();
                        task_picker.create(name).map(|_| ())
                    }

                    _ => continue,
                }
            }
        };

        match result {
            Err(error) => {
                ::ncurses::clear();
                ::ncurses::printw(error.description());
                ::ncurses::refresh();
                ::ncurses::getch();
            }
            _ => {}
        };

        ::ncurses::clear();
        ::ncurses::printw(&format!("{}", task_picker));
        ::ncurses::refresh();
    }

    ::ncurses::endwin();
    Ok(())
}

struct TaskPicker<T> {
    position: usize,
    tasks: T,
}

impl<T> TaskPicker<T>
    where T: TodoList,
          T::Id: Copy + From<usize>,
          usize: From<T::Id>,
          T::Id: From<usize>,
{
    fn down(&mut self) -> Result<(), T::Error> {
        let id = T::Id::from(self.position);
        self.position = match self.tasks.next_id(id) {
            Err(_) => self.position,
            Ok(i) => usize::from(i),
        };
        Ok(())
    }

    fn up(&mut self) -> Result<(), T::Error> {
        let id = T::Id::from(self.position);
        self.position = match self.tasks.next_back_id(id) {
            Err(_) => self.position,
            Ok(i) => usize::from(i),
        };
        Ok(())
    }

    fn right(&mut self) -> Result<(), T::Error> {
        let id = T::Id::from(self.position);
        let _ = self.tasks.update(id, Task::goto_next_status);
        Ok(())
    }

    fn left(&mut self) -> Result<(), T::Error> {
        let id = T::Id::from(self.position);
        let _ = self.tasks.update(id, Task::goto_next_back_status);
        Ok(())
    }

    fn create(&mut self, name: String) -> Result<T::Id, T::Error> {
        self.tasks.create(&name)
    }
}

impl<T> Display for TaskPicker<T>
    where T: TodoList
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut strings = Vec::new();
        self.tasks.enumerate(|id, task| {
            let marker = if id == self.position { ">" } else { " " };
            strings.push(format!("{} {}", marker, task));
        });
        write!(f, "  Wont Open Done\n{}", strings.join("\n"))
    }
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

#[derive(Debug)]
enum Status {
    Open,
    Done,
    Wont,
}

impl <'a> From<&'a str> for Status {
    fn from(source: &str) -> Status {
        match source {
            "Open" => Status::Open,
            "Done" => Status::Done,
            "Wont" => Status::Wont,
            _ => panic!("Invalid status string"),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Status::Open => write!(f, "     [ ]      "),
            Status::Done => write!(f, "           [x]"),
            Status::Wont => write!(f, "----          "),
        }
    }
}

#[derive(Debug)]
enum Error {
    AlreadyDone,
    AlreadyWont,
    Io(::std::io::Error),
    NoSuchTask,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", std::error::Error::description(self))
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::AlreadyDone => "The task is already finished",
            Error::AlreadyWont => "The task has already been closed",
            Error::Io(_) => "An IO error occured",
            Error::NoSuchTask => "No such task could be found",
        }
    }
}

impl From<::std::io::Error> for Error {

    fn from(cause: ::std::io::Error) -> Error {
        Error::Io(cause)
    }
}

trait TodoList {
    type Id: Copy;
    type Error: std::error::Error;

    fn create(&mut self, name: &str) -> Result<Self::Id, Self::Error>;

    fn each<F>(&self, f: F) where F: FnMut(&Task);

    fn update<F, R>(&mut self, id: Self::Id, f: F) -> Result<R, Self::Error>
        where F: FnOnce(&mut Task) -> R;

    fn enumerate<F>(&self, f: F) where F: FnMut(usize, &Task);

    fn next_id(&self, id: Self::Id) -> Result<Self::Id, Self::Error>;
    fn next_back_id(&self, id: Self::Id) -> Result<Self::Id, Self::Error>;

    fn create_finished(&mut self, name: &str) -> Result<Self::Id, Self::Error>
        where Self::Error: From<Error>
    {
        let id = self.create(name)?;
        try!(self.update(id, Task::finish));
        Ok(id)
    }

    fn create_closed(&mut self, name: &str) -> Result<Self::Id, Self::Error>
        where Self::Error: From<Error>
    {
        let id = self.create(name)?;
        try!(self.update(id, Task::close));
        Ok(id)
    }
}

impl Task {
    fn goto_next_status(&mut self) -> Result<(), Error> {
        self.status = match self.status {
            Status::Wont => Status::Open,
            Status::Open => Status::Done,
            Status::Done => return Err(Error::AlreadyDone),
        };
        Ok(())
    }

    fn goto_next_back_status(&mut self) -> Result<(), Error> {
        self.status = match self.status {
            Status::Open => Status::Wont,
            Status::Done => Status::Open,
            Status::Wont => return Err(Error::AlreadyWont),
        };
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Done => Err(Error::AlreadyDone),
            Status::Wont => Err(Error::AlreadyWont),
            _ => {
                self.status = Status::Done;
                Ok(())
            }
        }
    }

    fn close(&mut self) -> Result<(), Error> {
        match self.status {
            Status::Done => Err(Error::AlreadyDone),
            Status::Wont => Err(Error::AlreadyWont),
            _ => {
                self.status = Status::Wont;
                Ok(())
            }
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let check = format!("{}", self.status);
        write!(f, "{} {}", check, self.name)
    }
}

impl TodoList for FakeTodoList {
    type Id = usize;
    type Error = Error;

    fn create(&mut self, name: &str) -> Result<usize, Error> {
        let id = self.next_id;
        self.next_id += 1;

        let new_task = Task {
            status: Status::Open,
            name: String::from(name),
        };

        self.tasks.insert(id, new_task);
        Ok(id)
    }

    fn update<F, R>(&mut self, id: Self::Id, f: F) -> Result<R, Error>
        where F: FnOnce(&mut Task) -> R
    {
        Ok(f(&mut self.tasks[id]))
    }

    fn each<F>(&self, mut f: F)
        where F: FnMut(&Task)
    {
        for (_, task) in self.tasks.iter() {
            f(task);
        }
    }

    fn enumerate<F>(&self, mut f: F)
        where F: FnMut(usize, &Task)
    {
        for (id, task) in self.tasks.iter() {
            f(id, task);
        }
    }

    fn next_id(&self, id: Self::Id) -> Result<Self::Id, Error> {
        if id < self.next_id - 1 {
            Ok(id + 1)
        } else {
            Err(Error::NoSuchTask)
        }
    }

    fn next_back_id(&self, id: Self::Id) -> Result<Self::Id, Error> {
        if id > 0 {
            Ok(id - 1)
        } else {
            Err(Error::NoSuchTask)
        }
    }
}


struct FileTodoList {
    next_id: usize,
}

impl FileTodoList {
    fn new() -> FileTodoList {
        ::std::fs::DirBuilder::new()
            .recursive(true)
            .create(PATH).unwrap();

        let max_id = ::std::fs::read_dir(PATH).unwrap()
            .fold(0, |max, entry| {
                  let id = entry.unwrap()
                      .file_name()
                      .into_string().unwrap()
                      .parse().unwrap();
                  if id > max { id } else { max }
            });

        FileTodoList {
            next_id: max_id + 1,
        }
    }

    fn save(&mut self, id: usize, task: &Task) -> Result<(), ::std::io::Error> {
        let mut file = File::create(&format!("{}/{:05}", PATH, id))?;
        write!(file, "{}\n{:?}", task.name, task.status)
    }

    fn load(&self, id: usize) -> Result<Task, ::std::io::Error> {
        let mut file = File::open(&format!("{}/{:05}", PATH, id))?;
        let content = {
            let mut content = String::new();
            try!(file.read_to_string(&mut content));
            content
        };

        let lines = content.lines().collect::<Vec<_>>();
        assert_eq!(2, lines.len());
        Ok(Task {
            name: String::from(lines[0]),
            status: Status::from(lines[1]),
        })
    }

    fn ids(&self) -> Result<Vec<usize>, ::std::io::Error> {
        let mut ids = Vec::new();

        for entry in ::std::fs::read_dir(PATH)? {
            let name = entry?.file_name();
            let string = name.into_string().unwrap();
            let result = string.parse();
            ids.push(result.unwrap());
        }
        Ok(ids)
    }
}

impl TodoList for FileTodoList {
    type Id = usize;
    type Error = Error;

    fn create(&mut self, name: &str) -> Result<usize, Error> {
        let id = self.next_id;
        self.next_id += 1;

        let new_task = Task {
            status: Status::Open,
            name: String::from(name),
        };

        try!(self.save(id, &new_task));
        Ok(id)
    }

    fn update<F, R>(&mut self, id: Self::Id, f: F) -> Result<R, Error>
        where F: FnOnce(&mut Task) -> R
    {
        let mut task = self.load(id)?;
        let result = f(&mut task);
        try!(self.save(id, &task));
        Ok(result)
    }

    fn each<F>(&self, mut f: F)
        where F: FnMut(&Task)
    {
        for &id in self.ids().unwrap().iter() {
            let task = self.load(id).unwrap();
            f(&task);
        }
    }

    fn enumerate<F>(&self, mut f: F)
        where F: FnMut(usize, &Task)
    {
        for &id in self.ids().unwrap().iter() {
            let task = self.load(id).unwrap();
            f(id, &task);
        }
    }

    fn next_id(&self, id: Self::Id) -> Result<Self::Id, Error> {
        if id < self.next_id - 1 {
            Ok(id + 1)
        } else {
            Err(Error::NoSuchTask)
        }
    }

    fn next_back_id(&self, id: Self::Id) -> Result<Self::Id, Error> {
        if id > 1 {
            Ok(id - 1)
        } else {
            Err(Error::NoSuchTask)
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
