extern crate ncurses;
extern crate vec_map;

use std::fmt;
use std::fmt::{Display, Formatter};

use vec_map::VecMap;

fn main() {
    let mut todo_list = FakeTodoList::new();
    todo_list.create_finished("Start making ado").unwrap();
    todo_list.create_finished("Try rusqlite").unwrap();
    todo_list.create_closed("Implement a onion architecture").unwrap();
    todo_list.create_finished("Start simplified rewrite of ado").unwrap();
    todo_list.create("Refine the design of ado");
    todo_list.create_finished("Make ado interactive").unwrap();
    todo_list.create("Implement new task creation");
    todo_list.create("Implement persistence");
    todo_list.create_finished("Have ado use unbuffered input").unwrap();
    todo_list.create_finished("Eliminate the 'history' e.g. by redrawing the screen").unwrap();
    todo_list.create_finished("Implement task status toggling (done/open)").unwrap();
    todo_list.create_finished("Implement task status toggling (open/closed)").unwrap();
    todo_list.create_finished("Hide the cursor").unwrap();
    let mut task_picker: TaskPicker<FakeTodoList> = TaskPicker {
        position: 0,
        tasks: todo_list,
    };

    gui(&mut task_picker).unwrap();
}

fn gui<T>(task_picker: &mut TaskPicker<T>) -> Result<()>
where T: TodoList<IdType = usize>,
      T: Display
{
    use std::error::Error;

    ::ncurses::initscr();
    ::ncurses::noecho();
    ::ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    ::ncurses::printw(&format!("{}", task_picker));
    ::ncurses::refresh();

    loop {
        let result = match ::ncurses::getch() {
            x => match char::from(x as u8) {
                'q' => break,

                'h' => task_picker.left(),
                'j' => task_picker.down(),
                'k' => task_picker.up(),
                'l' => task_picker.right(),

                _ => continue,
            },
        };

        match result {
            Err(error) => {
                ::ncurses::clear();
                ::ncurses::printw(error.description());
                ::ncurses::refresh();
                ::ncurses::getch();
            },
            _ => {},
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

impl<T, I> TaskPicker<T>
where T: TodoList<IdType = I>,
      I: Copy + From<usize>,
      usize: From<I>,
      I: From<usize>,
{
    fn down(&mut self) -> Result<()> {
        self.position = match self.tasks.next_id(I::from(self.position)) {
            None => self.position,
            Some(i) => usize::from(i),
        };
        Ok(())
    }

    fn up(&mut self) -> Result<()> {
        self.position = match self.tasks.next_back_id(I::from(self.position)) {
            None => self.position,
            Some(i) => usize::from(i),
        };
        Ok(())
    }

    fn right(&mut self) -> Result<()> {
        let _ = self.tasks.update(I::from(self.position), Task::goto_next_status);
        Ok(())
    }

    fn left(&mut self) -> Result<()> {
        let _ = self.tasks.update(I::from(self.position), Task::goto_next_back_status);
        Ok(())
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
        write!(f, "  WONT TODO DONE\n{}", strings.join("\n"))
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

enum Status {
    Open,
    Done,
    Wont,
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

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum Error {
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
            Error::AlreadyDone => "The task is already finished",
            Error::AlreadyWont => "The task has already been closed",
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

    fn create_closed(&mut self, name: &str) -> Result<Self::IdType> {
        let id = self.create(name);
        try!(self.update(id, Task::close));
        Ok(id)
    }
}

impl Task {

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

    fn finish(&mut self) -> Result<()> {
        match self.status {
            Status::Done => Err(Error::AlreadyDone),
            Status::Wont => Err(Error::AlreadyWont),
            _ => {
                self.status = Status::Done;
                Ok(())
            }
        }
    }

    fn close(&mut self) -> Result<()> {
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
        for (id, task) in self.tasks.iter() {
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
