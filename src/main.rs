extern crate ncurses;
extern crate vec_map;

use ncurses::CURSOR_VISIBILITY;

use std::fmt;
use std::fmt::{Display, Formatter};

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;
use std::fs;
use std::ffi::OsString;

use vec_map::VecMap;

const PATH: &'static str = "./.ado/";

/// Constructs the application and runs the GUI.
fn main() {
    let todo_list = FileTodoList::new().unwrap();
    let mut task_picker = TaskPicker {
        position: 0,
        tasks: todo_list,
    };

    gui(&mut task_picker);
}

/// Handles input and output for the lifetime of the application.
///
/// The function initialises ncurses and the screen, then in a loop:
///
/// 1. updates the screen and
/// 2. handles user input.
///
/// This function returns when the user enters a quit command.
///
/// Partially completed commands are shown, but not complete
/// or invalid commands.
/// e.g. pressing 'd' will cause d to be printed at the bottom
/// of the screen until the command is completed (e.g. as 'dd')
/// or abandoned.
fn gui<T>(task_picker: &mut TaskPicker<T>)
    where T: TodoList<Id = usize, Error = Error>
{
    use ::std::error::Error;

    ::ncurses::initscr();
    ::ncurses::curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    ::ncurses::noraw();
    ::ncurses::cbreak();

    // Print the initial state of the task picker.
    ::ncurses::clear();
    ::ncurses::printw(&format!("{}\n", task_picker));
    ::ncurses::refresh();

    loop {
        // Handle user input, and store any errors which are produced.
        // Generate a new error if the input is unrecognised.
        let result = match ::ncurses::getch() {
            x => {
                match char::from(x as u8) {
                    // Quit on q.
                    'q' => break,

                    // Basic movement commands.
                    'h' => task_picker.left(),
                    'j' => task_picker.down(),
                    'k' => task_picker.up(),
                    'l' => task_picker.right(),

                    // Get a new task name from the user and use the
                    // name to create a new task.
                    'o' => {
                        ::ncurses::printw("\nEnter a task summary:\n");
                        ::ncurses::curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
                        ::ncurses::nocbreak();
                        let mut name = String::new();
                        ::ncurses::getstr(&mut name);
                        ::ncurses::curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
                        ::ncurses::cbreak();
                        task_picker.create(name).map(|_| ())
                    }

                    // Long distance scrolling.
                    'G' => task_picker.bottom(),
                    'g' => {
                        match char::from(::ncurses::getch() as u8) {
                            'g' => task_picker.top(),
                            _ => Err(self::Error::NoSuchCommand),
                        }
                    }

                    // Task deletion.
                    'D' => task_picker.remove(),
                    'd' => {
                        match char::from(::ncurses::getch() as u8) {
                            'd' => task_picker.remove(),
                            _ => Err(self::Error::NoSuchCommand),
                        }
                    }

                    _ => Err(self::Error::NoSuchCommand),
                }
            }
        };

        // Print the state of the task picker as well printing
        // any required error messages.
        ::ncurses::clear();
        ::ncurses::printw(&format!("{}\n", task_picker));
        if let Err(err) = result {
            ::ncurses::printw(&format!("{}\n", err.description()));
        }
        ::ncurses::refresh();
    }

    ::ncurses::endwin();
}

struct TaskPicker<T> {
    position: usize,
    tasks: T,
}

impl<T> TaskPicker<T>
    where T: TodoList,
          T::Id: PartialEq + Copy + From<usize>,
          usize: From<T::Id>,
          T::Error: From<Error>
{
    fn top(&mut self) -> Result<(), T::Error> {
        self.position = 0;
        Ok(())
    }

    fn bottom(&mut self) -> Result<(), T::Error> {
        let len = self.len()?;
        if len == 0 {
            Err(Error::NoSuchTask)?
        } else {
            self.position = len - 1;
            Ok(())
        }
    }

    fn down(&mut self) -> Result<(), T::Error> {
        let len = self.len()?;
        if len != 0 && len - 1 != self.position {
            self.position += 1;
        }
        Ok(())
    }

    fn len(&self) -> Result<usize, T::Error> {
        Ok(self.tasks.ids().collect::<Vec<_>>().len())
    }

    fn up(&mut self) -> Result<(), T::Error> {
        if self.position != 0 {
            self.position -= 1;
        }
        Ok(())
    }

    fn right(&mut self) -> Result<(), T::Error> {
        let id = self.current_id()?;
        Ok(try!(self.tasks.find_mut(id)?
            .goto_next_status()))
    }

    fn left(&mut self) -> Result<(), T::Error> {
        let id = self.current_id()?;
        Ok(try!(self.tasks.find_mut(id)?
            .goto_next_back_status()))
    }

    fn current_id(&self) -> Result<T::Id, T::Error> {
        match self.tasks.ids().nth(self.position) {
            None => Err(Error::NoSuchTask)?,
            Some(id) => Ok(id?),
        }
    }

    fn create(&mut self, name: String) -> Result<T::Id, T::Error> {
        let new_id = self.tasks.create(&name)?;
        let mut new_position = self.position;
        for (p, id) in self.tasks.ids().enumerate() {
            match id {
                Ok(id) if id == new_id => new_position = p,
                _ => (),
            };
        }
        self.position = new_position;
        Ok(new_id)
    }

    fn remove(&mut self) -> Result<(), T::Error> {
        let id = self.tasks
            .ids()
            .nth(self.position)
            .unwrap_or(Err(<_>::from(Error::NoSuchTask)))?;

        // Make sure we will still have our cursor in a valid position.
        let new_len = self.len()? - 1;
        if self.position >= new_len && new_len > 0 {
            self.position = new_len - 1;
        }

        try!(self.tasks.remove(id));

        Ok(())
    }
}

impl<T, I> Display for TaskPicker<T>
    where T: TodoList<Id = I>,
          I: Copy + From<usize> + PartialEq + Display
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut strings = Vec::new();
        let current_id = self.tasks
            .ids()
            .nth(self.position)
            .unwrap_or(Ok(<_>::from(0)))
            .unwrap_or(<_>::from(0));
        // TODO report errors instead of flat_mapping.
        for (id, task) in self.tasks.enumerate().flat_map(Result::ok) {
            let marker = if id == current_id { ">" } else { " " };
            strings.push(format!("{} {}", marker, task));
        }
        write!(f, "  Wont Open Done\n{}", strings.join("\n"))
    }
}

pub struct FakeTodoList {
    tasks: VecMap<Task>,
    next_id: usize,
}

impl FakeTodoList {
    pub fn new() -> FakeTodoList {
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

impl<'a> From<&'a str> for Status {
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
pub enum Error {
    AlreadyDone,
    AlreadyWont,
    Io(::std::io::Error),
    NoSuchTask,
    NoSuchCommand,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", std::error::Error::description(self))
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::AlreadyDone => "The task is already finished",
            Error::AlreadyWont => "The task has already been closed",
            Error::Io(_) => "An IO error occured",
            Error::NoSuchTask => "No such task could be found",
            Error::NoSuchCommand => "Unrecognised command",
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

    fn iter<'a>(&'a self) -> Box<Iterator<Item = Result<&'a Task, Self::Error>> + 'a>;
    fn iter_mut<'a>(&'a mut self) -> Box<Iterator<Item = Result<&'a mut Task, Self::Error>> + 'a>;
    fn into_iter(self) -> Box<Iterator<Item = Result<Task, Self::Error>>>;

    fn enumerate<'a>(&'a self) -> Box<Iterator<Item = Result<(Self::Id, &'a Task), Self::Error>> + 'a>;
    fn ids<'a>(&'a self) -> Box<Iterator<Item = Result<Self::Id, Self::Error>> + 'a>;

    fn find(&self, id: Self::Id) -> Result<&Task, Self::Error>;
    fn find_mut(&mut self, id: Self::Id) -> Result<&mut Task, Self::Error>;
    fn remove(&mut self, id: Self::Id) -> Result<Task, Self::Error>;
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

    fn enumerate<'a>(&'a self) -> Box<Iterator<Item = Result<(Self::Id, &'a Task), Self::Error>> + 'a> {
        Box::new(self.tasks.iter()
                 .map(|pair| Ok(pair)))
    }

    fn ids<'a>(&'a self) -> Box<Iterator<Item = Result<Self::Id, Self::Error>> + 'a> {
        Box::new(self.tasks.keys()
                 .map(|key| Ok(key)))
    }

    fn remove(&mut self, id: Self::Id) -> Result<Task, Self::Error> {
        self.tasks
            .remove(id)
            .map_or(Err(Error::NoSuchTask), |task| Ok(task))
    }

    fn find(&self, id: Self::Id) -> Result<&Task, Self::Error> {
        Ok(&self.tasks[id])
    }

    fn find_mut(&mut self, id: Self::Id) -> Result<&mut Task, Self::Error> {
        Ok(&mut self.tasks[id])
    }

    fn iter<'a>(&'a self) -> Box<Iterator<Item = Result<&'a Task, Self::Error>> + 'a> {
        let iter = self.tasks.iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn iter_mut<'a>(&'a mut self) -> Box<Iterator<Item = Result<&'a mut Task, Self::Error>> + 'a> {
        let iter = self.tasks.iter_mut()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn into_iter(self) -> Box<Iterator<Item = Result<Task, Self::Error>>> {
        let iter = self.tasks.into_iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }
}

/// A Task source backed by flat files.
pub struct FileTodoList {
    next_id: usize,
    cache: HashMap<usize, Task>,
}

impl FileTodoList {
    pub fn new() -> Result<FileTodoList, Error> {
        ::std::fs::DirBuilder::new()
            .recursive(true)
            .create(PATH)
            .unwrap();

        let mut todo_list = FileTodoList {
            next_id: ids()?.max().unwrap_or(0) + 1,
            cache: HashMap::new(),
        };
        try!(todo_list.load_all());
        Ok(todo_list)
    }

    fn save(&mut self, id: usize, task: Task) -> Result<(), ::std::io::Error> {
        let mut file = File::create(&format!("{}/{:05}", PATH, id))?;
        let content = format!("{}\n{:?}", task.name, task.status);
        self.cache.insert(id, task);
        write!(file, "{}", content)
    }

    fn load_all(&mut self) -> Result<(), ::std::io::Error> {
        for id in ids()? {
            let task = self.load(id)?;
            match self.cache.insert(id, task) {
                // TODO handle this gracefully
                Some(_) => panic!("Loaded the same task twice"),
                _ => {},
            };
        }
        Ok(())
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

        try!(self.save(id, new_task));
        Ok(id)
    }

    fn enumerate<'a>(&'a self) -> Box<Iterator<Item = Result<(Self::Id, &'a Task), Self::Error>> + 'a> {
        match ids() {
            Ok(ids) => Box::new(ids.map(move |id| {
                let task = &self.cache[&id];
                Ok((id, task))
            })),
            Err(err) => Box::new(vec![Err(<_>::from(err))].into_iter()),
        }
    }

    fn ids<'a>(&'a self) -> Box<Iterator<Item = Result<Self::Id, Self::Error>> + 'a> {
        match ids() {
            Ok(ids) => Box::new(ids.map(|x| Ok(x))),
            Err(err) => Box::new(vec![Err(<_>::from(err))].into_iter()),
        }
    }

    fn remove(&mut self, id: Self::Id) -> Result<Task, Self::Error> {
        // Load the task first so it can be moved out.
        let task = self.load(id)?;

        fs::remove_file(&format!("{}/{:05}", PATH, id))?;

        Ok(task)
    }

    fn find(&self, id: Self::Id) -> Result<&Task, Self::Error> {
        Ok(&self.cache[&id])
    }

    fn find_mut(&mut self, id: Self::Id) -> Result<&mut Task, Self::Error> {
        match self.cache.get_mut(&id) {
            None => Err(Error::NoSuchTask),
            Some(task) => Ok(task),
        }
    }

    fn iter<'a>(&'a self) -> Box<Iterator<Item = Result<&'a Task, Self::Error>> + 'a> {
        let iter = self.cache.iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn iter_mut<'a>(&'a mut self) -> Box<Iterator<Item = Result<&'a mut Task, Self::Error>> + 'a> {
        let iter = self.cache.iter_mut()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn into_iter(self) -> Box<Iterator<Item = Result<Task, Self::Error>>> {
        let iter = self.cache.into_iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }
}

fn ids() -> Result<Box<Iterator<Item = usize>>, ::std::io::Error> {
    let read_dir = ::std::fs::read_dir(PATH)?;

    // TODO: Report errors in some way.
    // Get a usize for each filename in the data path, where possible.
    let mut ids = read_dir.flat_map(Result::ok)
        .map(|entry| entry.file_name())
        .flat_map(OsString::into_string)
        .flat_map(|name| name.parse())
        .collect::<Vec<_>>();

    ids.sort();
    Ok(Box::new(ids.into_iter()))
}
