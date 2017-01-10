extern crate ncurses;
extern crate vec_map;

use ncurses::CURSOR_VISIBILITY;

use std::fmt;
use std::fmt::{Display, Formatter};

use std::cmp;
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
    where T: TodoList<Error = Error>,
          T::Error: From<<T::Task as Task>::Error>
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
          Error: From<T::Error>,
          Error: From<<T::Task as Task>::Error>
{
    fn top(&mut self) -> Result<()> {
        self.position = 0;
        Ok(())
    }

    fn bottom(&mut self) -> Result<()> {
        let len = self.len()?;
        if len == 0 {
            Err(Error::NoSuchTask)?
        } else {
            self.position = len - 1;
            Ok(())
        }
    }

    fn down(&mut self) -> Result<()> {
        let len = self.len()?;
        if len != 0 && len - 1 != self.position {
            self.position += 1;
        }
        Ok(())
    }

    fn len(&self) -> Result<usize> {
        Ok(self.tasks.ids().collect::<Vec<_>>().len())
    }

    fn up(&mut self) -> Result<()> {
        if self.position != 0 {
            self.position -= 1;
        }
        Ok(())
    }

    fn right(&mut self) -> Result<()> {
        let id = self.current_id()?;
        self.tasks
            .find_mut(id)?
            .goto_next_status()
            .map_err(Error::from)
    }

    fn left(&mut self) -> Result<()> {
        let id = self.current_id()?;
        self.tasks
            .find_mut(id)?
            .goto_next_back_status()
            .map_err(Error::from)
    }

    fn current_id(&self) -> Result<usize> {
        self.tasks
            .ids()
            .nth(self.position)
            .map(|result| result.map_err(Error::from))
            .unwrap_or(Err(Error::NoSuchTask))
    }

    fn create(&mut self, name: String) -> Result<usize> {
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

    fn remove(&mut self) -> Result<()> {
        let id = self.tasks
            .ids()
            .nth(self.position)
            .map(|result| result.map_err(Error::from))
            .unwrap_or(Err(Error::NoSuchTask))?;

        try!(self.tasks.remove(id));

        // Make sure we will still have our cursor in a valid position.
        self.position = cmp::min(self.position, cmp::max(1, self.len()?) - 1);
        Ok(())
    }
}

impl<T> Display for TaskPicker<T>
    where T: TodoList
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut strings = Vec::new();

        // TODO report errors instead of flat_mapping.
        for (position, task) in self.tasks.sorted().enumerate() {
            let task = task.map_err(|_| fmt::Error)?;
            let marker = if position == self.position { ">" } else { " " };
            strings.push(format!("{} {}", marker, task.projection()));
        }

        write!(f, "  Wont Open Done\n{}", strings.join("\n"))
    }
}

pub struct FakeTodoList {
    tasks: VecMap<BasicTask>,
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

#[derive(Clone)]
pub struct BasicTask {
    status: Status,
    name: String,
}

impl Task for BasicTask {
    type Error = Error;

    fn goto_next_status(&mut self) -> Result<()> {
        self.status = match self.status {
            Status::Wont => Status::Open,
            Status::Open => Status::Done,
            Status::Done => return Err(Error::AlreadyDone),
        };
        Ok(())
    }

    fn goto_next_back_status(&mut self) -> Result<()> {
        self.status = match self.status {
            Status::Open => Status::Wont,
            Status::Done => Status::Open,
            Status::Wont => return Err(Error::AlreadyWont),
        };
        Ok(())
    }

    fn projection(&self) -> BasicTask {
        BasicTask { ..self.clone() }
    }
}

#[derive(Debug, Clone)]
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
    type Error: std::error::Error + From<<Self::Task as Task>::Error>;
    type Task: Task;

    fn create(&mut self, name: &str) -> Result<usize, Self::Error>;

    fn iter(&self) -> ResultIter<&Self::Task, Self::Error>;
    fn iter_mut(&mut self) -> ResultIter<&mut Self::Task, Self::Error>;
    fn into_iter<'a>(self) -> ResultIter<'a, Self::Task, Self::Error>;

    fn enumerate(&self) -> ResultIter<(usize, &Self::Task), Self::Error>;
    fn ids(&self) -> ResultIter<usize, Self::Error> {
        // By default we can drop the tasks from the enumerate output.
        Box::new(
            self.enumerate()
            .map(|result| result.map(|(id, _)| id)))
    }
    fn sorted(&self) -> ResultIter<&Self::Task, Self::Error> {
        // By default we can drop the ids from the enumerate output.
        Box::new(
            self.enumerate()
            .map(|result| result.map(|(_, task)| task)))
    }

    fn find(&self, id: usize) -> Result<&Self::Task, Self::Error>;
    fn find_mut(&mut self, id: usize) -> Result<&mut Self::Task, Self::Error>;
    fn remove(&mut self, id: usize) -> Result<Self::Task, Self::Error>;
}

pub trait Task {
    type Error: std::error::Error;

    fn goto_next_status(&mut self) -> Result<(), Self::Error>;
    fn goto_next_back_status(&mut self) -> Result<(), Self::Error>;

    fn projection(&self) -> BasicTask;
}

impl Display for BasicTask {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let check = format!("{}", self.status);
        write!(f, "{} {}", check, self.name)
    }
}

impl TodoList for FakeTodoList {
    type Error = Error;
    type Task = BasicTask;

    fn create(&mut self, name: &str) -> Result<usize, Error> {
        let id = self.next_id;
        self.next_id += 1;

        let new_task = BasicTask {
            status: Status::Open,
            name: String::from(name),
        };

        self.tasks.insert(id, new_task);
        Ok(id)
    }

    fn enumerate(&self) -> ResultIter<(usize, &Self::Task)> {
        Box::new(self.tasks
            .iter()
            .map(|pair| Ok(pair)))
    }

    fn remove(&mut self, id: usize) -> Result<Self::Task> {
        self.tasks
            .remove(id)
            .map_or(Err(Error::NoSuchTask), |task| Ok(task))
    }

    fn find(&self, id: usize) -> Result<&Self::Task> {
        Ok(&self.tasks[id])
    }

    fn find_mut(&mut self, id: usize) -> Result<&mut Self::Task> {
        Ok(&mut self.tasks[id])
    }

    fn iter(&self) -> ResultIter<&Self::Task> {
        let iter = self.tasks
            .iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn iter_mut(&mut self) -> ResultIter<&mut Self::Task> {
        let iter = self.tasks
            .iter_mut()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn into_iter<'a>(self) -> ResultIter<'a, Self::Task> {
        let iter = self.tasks
            .into_iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }
}

/// A Task source backed by flat files.
pub struct FileTodoList {
    cache: HashMap<usize, FileTask<BasicTask>>,
    ids: Vec<usize>,
}

pub struct FileTask<T = BasicTask> {
    file_name: String,
    inner: T,
}

impl<T> FileTask<T>
    where T: Task,
          Error: From<T::Error>
{
    fn new(inner: T, file_name: String) -> Result<FileTask<T>, ::std::io::Error> {
        let task = FileTask {
            inner: inner,
            file_name: file_name,
        };
        try!(task.save());
        Ok(task)
    }

    fn save(&self) -> Result<(), ::std::io::Error> {
        let mut file = File::create(&self.file_name)?;
        let projection = self.projection();
        write!(file, "{}\n{:?}", projection.name, projection.status)
    }
}

impl<T> Task for FileTask<T>
    where T: Task,
          Error: From<T::Error>
{
    type Error = Error;

    fn goto_next_status(&mut self) -> Result<(), Error> {
        try!(self.inner.goto_next_status());
        self.save().map_err(Error::Io)
    }

    fn goto_next_back_status(&mut self) -> Result<(), Error> {
        try!(self.inner.goto_next_back_status());
        self.save().map_err(Error::Io)
    }

    fn projection(&self) -> BasicTask {
        BasicTask { ..self.inner.projection() }
    }
}

impl FileTodoList {
    pub fn new() -> Result<FileTodoList> {
        ::std::fs::DirBuilder::new()
            .recursive(true)
            .create(PATH)
            .unwrap();

        let mut todo_list = FileTodoList {
            ids: ids()?,
            cache: HashMap::new(),
        };
        try!(todo_list.load_all());
        Ok(todo_list)
    }

    fn load_all(&mut self) -> Result<(), ::std::io::Error> {
        for &id in self.ids.iter() {
            let task = Self::load(id)?;
            match self.cache.insert(id, task) {
                // TODO handle this gracefully
                Some(_) => panic!("Loaded the same task twice"),
                _ => {}
            };
        }
        Ok(())
    }

    fn file_name(id: usize) -> String {
        format!("{}/{:05}", PATH, id)
    }

    fn load(id: usize) -> Result<FileTask, ::std::io::Error> {
        let mut file = File::open(&Self::file_name(id))?;
        let content = {
            let mut content = String::new();
            try!(file.read_to_string(&mut content));
            content
        };

        let lines = content.lines().collect::<Vec<_>>();
        assert_eq!(2, lines.len());

        let inner = BasicTask {
            name: String::from(lines[0]),
            status: Status::from(lines[1]),
        };
        Ok(FileTask {
            file_name: Self::file_name(id),
            inner: inner,
        })
    }
}

type ResultIter<'a, T, E = Error> = Box<Iterator<Item = Result<T, E>> + 'a>;
type Result<T, E = Error> = std::result::Result<T, E>;

impl TodoList for FileTodoList {
    type Error = Error;
    type Task = FileTask;

    fn create(&mut self, name: &str) -> Result<usize> {
        let id = self.ids.last().unwrap_or(&0) + 1;

        let inner = BasicTask {
            status: Status::Open,
            name: String::from(name),
        };

        let new_task = FileTask::new(inner, Self::file_name(id))?;

        if let Some(_) = self.cache.insert(id, new_task) {
            // TODO Handle gracefully
            panic!("Created preexisting task");
        }
        Ok(id)
    }

    fn enumerate(&self) -> ResultIter<(usize, &Self::Task)> {
        Box::new(
            self.ids
            .iter()
            .map(move |&id| Ok((id, &self.cache[&id]))))
    }

    fn remove(&mut self, id: usize) -> Result<Self::Task> {
        // Load the task first so it can be moved out.
        let task = Self::load(id)?;

        fs::remove_file(&format!("{}/{:05}", PATH, id))?;

        Ok(task)
    }

    fn find(&self, id: usize) -> Result<&Self::Task> {
        Ok(&self.cache[&id])
    }

    fn find_mut(&mut self, id: usize) -> Result<&mut Self::Task> {
        match self.cache.get_mut(&id) {
            None => Err(Error::NoSuchTask),
            Some(task) => Ok(task),
        }
    }

    fn iter(&self) -> ResultIter<&Self::Task> {
        let iter = self.cache
            .iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn iter_mut(&mut self) -> ResultIter<&mut Self::Task> {
        let iter = self.cache
            .iter_mut()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }

    fn into_iter<'a>(self) -> ResultIter<'a, Self::Task> {
        let iter = self.cache
            .into_iter()
            .map(|(_, task)| Ok(task));
        Box::new(iter)
    }
}

fn ids() -> Result<Vec<usize>, ::std::io::Error> {
    let read_dir = ::std::fs::read_dir(PATH)?;

    // TODO: Report errors in some way instead of swallowing them in flat_map.
    // Get a usize for each file name in the data path, where possible.
    let mut ids = read_dir.flat_map(Result::ok)
        .map(|entry| entry.file_name())
        .flat_map(OsString::into_string)
        .flat_map(|name| name.parse())
        .collect::<Vec<_>>();

    ids.sort();
    Ok(ids)
}
