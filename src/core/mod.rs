use vec_map::VecMap;

use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    AlreadyDone,
    AlreadyWont,
    External(Box<::std::error::Error>),
    NoSuchTask,
}

impl From<::std::fmt::Error> for Error {
    fn from(cause: ::std::fmt::Error) -> Error {
        Error::External(Box::new(cause))
    }
}

impl From<::std::io::Error> for Error {
    fn from(cause: ::std::io::Error) -> Error {
        Error::External(Box::new(cause))
    }
}

pub type ResultIter<'a, T, E = Error> = Box<Iterator<Item = Result<T, E>> + 'a>;
pub type Result<T, E = Error> = ::std::result::Result<T, E>;

pub trait TodoList {
    type Error: ::std::error::Error + From<<Self::Task as Task>::Error>;
    type Task: Task;

    fn create(&mut self, name: &str) -> Result<usize, Self::Error>;

    fn iter(&self) -> ResultIter<&Self::Task, Self::Error>;
    fn iter_mut(&mut self) -> ResultIter<&mut Self::Task, Self::Error>;
    fn into_iter<'a>(self) -> ResultIter<'a, Self::Task, Self::Error>;

    /// Enumerate, ids, and sorted, must produce results in
    /// an order which is consistent with eachother.
    fn enumerate(&self) -> ResultIter<(usize, &Self::Task), Self::Error>;
    fn ids(&self) -> ResultIter<usize, Self::Error> {
        // By default we can drop the tasks from the enumerate output.
        Box::new(self.enumerate()
            .map(|result| result.map(|(id, _)| id)))
    }
    fn sorted(&self) -> ResultIter<&Self::Task, Self::Error> {
        // By default we can drop the ids from the enumerate output.
        Box::new(self.enumerate()
            .map(|result| result.map(|(_, task)| task)))
    }

    fn find(&self, id: usize) -> Result<&Self::Task, Self::Error>;
    fn find_mut(&mut self, id: usize) -> Result<&mut Self::Task, Self::Error>;
    fn remove(&mut self, id: usize) -> Result<Self::Task, Self::Error>;
}

pub trait Task {
    type Error: ::std::error::Error;

    fn goto_next_status(&mut self) -> Result<(), Self::Error>;
    fn goto_next_back_status(&mut self) -> Result<(), Self::Error>;

    fn projection(&self) -> BasicTask;
}

#[derive(Clone)]
pub struct BasicTask {
    pub status: Status,
    pub name: String,
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
            Status::Wont => return Err(Error::AlreadyDone),
        };
        Ok(())
    }

    fn projection(&self) -> BasicTask {
        BasicTask { ..self.clone() }
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Open,
    Done,
    Wont,
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", ::std::error::Error::description(self))
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::AlreadyDone => "The task is already finished",
            Error::AlreadyWont => "The task has already been closed",
            Error::NoSuchTask => "No such task could be found",
            Error::External(_) => "An external error occured",
        }
    }
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

impl Display for BasicTask {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let check = format!("{}", self.status);
        write!(f, "{} {}", check, self.name)
    }
}
