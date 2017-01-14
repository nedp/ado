extern crate ncurses;
extern crate ado;

use ncurses::CURSOR_VISIBILITY;

use std::fmt;
use std::fmt::{Display, Formatter};

use std::cmp;
use std::collections::HashMap;
use std::fs::File;
use std::fs;
use std::ffi::OsString;
use std::io::prelude::*;

use ado::{BasicTask, Error, ResultIter, Status, Task, TodoList};

const PATH: &'static str = "./.ado/";

type FrontResult<T> = ::std::result::Result<T, FrontError>;

#[derive(Debug)]
enum FrontError {
    NoSuchCommand,
    Ado(Error),
}

impl From<Error> for FrontError {
    fn from(source: Error) -> FrontError {
        FrontError::Ado(source)
    }
}

impl Display for FrontError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", ::std::error::Error::description(self))
    }
}

impl ::std::error::Error for FrontError {
    fn description(&self) -> &str {
        match *self {
            FrontError::NoSuchCommand => "Command not recognised",
            FrontError::Ado(ref cause) => cause.description(),
        }
    }
}

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
          T::Error: From<<T::Task as Task>::Error>,
          FrontError: From<<T::Task as Task>::Error>
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
                            _ => Err(FrontError::NoSuchCommand),
                        }
                    }

                    // Task deletion.
                    'D' => task_picker.remove(),
                    'd' => {
                        match char::from(::ncurses::getch() as u8) {
                            'd' => task_picker.remove(),
                            _ => Err(FrontError::NoSuchCommand),
                        }
                    }

                    _ => Err(FrontError::NoSuchCommand),
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
          FrontError: From<T::Error>,
          FrontError: From<<T::Task as Task>::Error>,
          ado::Error: From<T::Error>,
          ado::Error: From<<T::Task as Task>::Error>
{
    fn top(&mut self) -> FrontResult<()> {
        self.position = 0;
        Ok(())
    }

    fn bottom(&mut self) -> FrontResult<()> {
        let len = self.len()?;
        if len == 0 {
            Err(Error::NoSuchTask)?
        } else {
            self.position = len - 1;
            Ok(())
        }
    }

    fn down(&mut self) -> FrontResult<()> {
        let len = self.len()?;
        if len != 0 && len - 1 != self.position {
            self.position += 1;
        }
        Ok(())
    }

    fn len(&self) -> FrontResult<usize> {
        Ok(self.tasks.ids().collect::<Vec<_>>().len())
    }

    fn up(&mut self) -> FrontResult<()> {
        if self.position != 0 {
            self.position -= 1;
        }
        Ok(())
    }

    fn right(&mut self) -> FrontResult<()> {
        let id = self.current_id()?;
        self.tasks
            .find_mut(id)?
            .goto_next_status()
            .map_err(FrontError::from)
    }

    fn left(&mut self) -> FrontResult<()> {
        let id = self.current_id()?;
        self.tasks
            .find_mut(id)?
            .goto_next_back_status()
            .map_err(FrontError::from)
    }

    fn current_id(&self) -> FrontResult<usize> {
        self.tasks
            .ids()
            .nth(self.position)
            .map(|result| result.map_err(Error::from))
            .unwrap_or(Err(Error::NoSuchTask))
            .map_err(FrontError::from)
    }

    fn create(&mut self, name: String) -> FrontResult<usize> {
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

    fn remove(&mut self) -> FrontResult<()> {
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

    fn save_map_err<E>(&self) -> Result<(), E>
        where E: From<::std::io::Error>
    {
        self.save()
            .map_err(E::from)
    }
}

impl<T> Task for FileTask<T>
    where T: Task,
          Error: From<T::Error>
{
    type Error = Error;

    fn goto_next_status(&mut self) -> Result<(), Error> {
        try!(self.inner.goto_next_status());
        self.save_map_err()
    }

    fn goto_next_back_status(&mut self) -> Result<(), Error> {
        try!(self.inner.goto_next_back_status());
        self.save_map_err()
    }

    fn projection(&self) -> BasicTask {
        BasicTask { ..self.inner.projection() }
    }
}

impl FileTodoList {
    pub fn new() -> ado::Result<FileTodoList> {
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

impl TodoList for FileTodoList {
    type Error = Error;
    type Task = FileTask;

    fn create(&mut self, name: &str) -> ado::Result<usize> {
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
        self.ids.push(id);

        Ok(id)
    }

    fn enumerate(&self) -> ResultIter<(usize, &Self::Task)> {
        Box::new(self.ids
            .iter()
            .map(move |&id| Ok((id, &self.cache[&id]))))
    }

    fn remove(&mut self, id: usize) -> ado::Result<Self::Task> {
        // Fail fast if our file access is broken.
        fs::remove_file(&format!("{}/{:05}", PATH, id))?;

        // Load the task and remove it from the cache.
        let index = self.ids.binary_search(&id)
            .map_err(|_| Error::NoSuchTask)?;
        self.ids.remove(index);

        // If the task isn't present, the index search should have failed.
        let task = self.cache.remove(&id).unwrap();
        Ok(task)
    }

    fn find(&self, id: usize) -> ado::Result<&Self::Task> {
        Ok(&self.cache[&id])
    }

    fn find_mut(&mut self, id: usize) -> ado::Result<&mut Self::Task> {
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
