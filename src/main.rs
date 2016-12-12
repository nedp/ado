use std::fmt;
use std::fmt::{Display, Formatter};
use std::iter::Map;
use std::slice::Iter;

use self::Status::{Todo, Done};

fn main() {
    test(FakeTodoList::new());
}

fn test<T>(mut tasks: T)
    where T: TodoList + Display {
    print!("\n=== Initial ===\n");

    let start = tasks.add(String::from("Start making ado"));
    let rusqlite = tasks.add(String::from("Try rusqlite"));
    let onion = tasks.add(String::from("Implement an onion architecture"));
    let rewrite = tasks.add(String::from("Start a simplified rewrite of ado"));
    tasks.add(String::from("Refine the design of ado"));

    println!("{}", tasks);

    print!("\n=== Now ===\n");

    tasks.finish(start);
    tasks.finish(rusqlite);
    tasks.finish(rewrite);

    println!("{}", tasks);

    print!("\n=== After ===\n");

    tasks.each(Task::finish);

    println!("{}", tasks);

    print!("\n=== Clean ===\n");

    tasks.remove(onion);

    println!("{}", tasks);
}

struct FakeTodoList(Vec<Task>);

impl FakeTodoList {
    fn new() -> FakeTodoList {
        FakeTodoList(Vec::new())
    }
}


struct Task {
    status: Status,
    name: String,
}

enum Status {
    Todo,
    Done,
}

trait TodoList {
    type IdType;

    fn add(&mut self, name: String) -> Self::IdType;
    fn remove(&mut self, id: Self::IdType) -> Task;
    fn update<F>(&mut self, id: Self::IdType, f: F) where F: FnOnce(&mut Task);
    fn each<F>(&mut self, mut f: F) where F: FnMut(&mut Task);
    fn map <F, R>(&self, f: F) -> Map<Iter<Task>, F> where F: FnMut(&Task) -> R;
    fn finish(&mut self, id: Self::IdType);
}

impl Task {
    fn finish(&mut self) {
        self.status = Done;
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let check = match self.status {
            Todo => "[ ]",
            Done => "[x]",
        };
        write!(f, "{} {}", check, self.name)
    }
}

impl TodoList for FakeTodoList {
    type IdType = usize;

    fn add(&mut self, name: String) -> Self::IdType {
        self.0.push(Task {
            status: Todo,
            name: name,
        });
        self.0.len()
    }

    fn remove(&mut self, id: Self::IdType) -> Task {
        self.0.remove(id)
    }

    fn update<F>(&mut self, id: Self::IdType, f: F) where F: FnOnce(&mut Task) {
        f(&mut self.0[id]);
    }

    fn each<F>(&mut self, mut f: F) where F: FnMut(&mut Task) {
        for i in 0..self.0.len() {
            self.update(i, &mut f);
        }
    }

    fn map <F, R>(&self, f: F) -> Map<Iter<Task>, F>
    where F: FnMut(&Task) -> R {
        self.0.iter().map(f)
    }

    fn finish(&mut self, id: Self::IdType) {
        self.0[id].finish();
    }
}

impl Display for FakeTodoList {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let joined = self.map(|task| format!("{}", task)).collect::<Vec<_>>().join("\n");
        write!(f, "{}", joined)
    }
}

