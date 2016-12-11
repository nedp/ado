use std::fmt;
use std::fmt::{Display, Formatter};


use self::Task::{Todo, Done};

enum Task {
    Todo(String),
    Done(String),
}

impl Task {
    fn finish(self) -> Self {
        match self {
            Todo(name) | Done(name) => Done(name),
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            &Todo(ref name) => write!(f, "[ ] {}", name),
            &Done(ref name) => write!(f, "[x] {}", name),
        }
    }
}

fn main() {
    print!("\n=== Initial ===\n");

    let mut tasks = TaskSource(Vec::new());
    let start = tasks.add(String::from("Start making ado"));
    let rusqlite = tasks.add(String::from("Try rusqlite"));
    let onion = tasks.add(String::from("Implement an onion architecture"));
    let rewrite = tasks.add(String::from("Start a simplified rewrite of ado"));
    let refine = tasks.add(String::from("Refine the design of ado"));

    tasks = tasks.map(|task| {
        println!("{}", task);
        task
    });

    print!("\n=== Now ===\n");

    tasks.update(start, Task::finish);
    tasks.update(rusqlite, Task::finish);
    tasks.update(rewrite, Task::finish);

    tasks = tasks.map(|task| {
        println!("{}", task);
        task
    });

    print!("\n=== After ===\n");

    tasks = tasks.map(Task::finish);

    tasks.map(|task| {
        println!("{}", task);
        task
    });
}

struct TaskSource(Vec<Task>);

type IdType = usize;

impl TaskSource {
    fn add(&mut self, name: String) -> IdType {
        self.0.push(Todo(name));
        self.0.len()
    }

    fn update<F>(&mut self, id: IdType, f: F) where F: FnOnce(Task) -> Task {
        let task = self.0.swap_remove(id);
        self.0.push(f(task));
        let len = self.0.len();
        self.0.swap(id, len - 1);
    }

    fn map<F>(self, f: F) -> Self where F: FnMut(Task) -> Task {
        let new_tasks = self.0.into_iter().map(f).collect::<Vec<_>>();
        TaskSource(new_tasks)
    }
}
