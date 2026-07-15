use clap::Parser;
use chrono::{Local, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Parser)]
struct Cli {
    command: String,
    task: Option<String>,
    subtask: Option<String>,

    #[arg(short, long)]
    start: Option<String>,

    #[arg(short, long)]
    end: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubTask {
    name: String,
    is_done: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    title: String,
    subtasks: Vec<SubTask>,
    start_time: Option<String>,
    end_time: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TodoList {
    items: HashMap<String, Task>,
}

impl TodoList {
    fn new() -> TodoList {
        TodoList {
            items: HashMap::new(),
        }
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> TodoList {
        if !path.as_ref().exists() {
            return TodoList::new();
        }
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return TodoList::new(),
        };
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap_or_else(|_| TodoList::new())
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    fn add_task(
        &mut self,
        title: String,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Result<(), String> {
        let lookup_key = title.to_lowercase();
        
        // Quick input formatting validation check if user supplied end_time
        if let Some(ref end) = end_time {
            if Self::parse_time_flexibly(end).is_none() {
                return Err("Invalid time format! Use formats like '6:30 PM', '18:30', or '6:30pm'".to_string());
            }
        }

        if let Entry::Vacant(entry) = self.items.entry(lookup_key) {
            entry.insert(Task {
                title,
                subtasks: Vec::new(),
                start_time,
                end_time,
            });
            Ok(())
        } else {
            Err("Task already exists!".to_string())
        }
    }

    fn add_subtask(&mut self, parent_task: &str, subtask_name: String) -> Result<(), String> {
        let lookup_key = parent_task.to_lowercase();
        if let Some(task) = self.items.get_mut(&lookup_key) {
            if task.subtasks.iter().any(|s| s.name.to_lowercase() == subtask_name.to_lowercase()) {
                return Err("Subtask already exists in this task!".to_string());
            }
            task.subtasks.push(SubTask {
                name: subtask_name,
                is_done: false,
            });
            Ok(())
        } else {
            Err(format!("Parent task '{}' not found.", parent_task))
        }
    }

    fn mark_subtask_done(&mut self, parent_task: &str, subtask_name: &str) -> Result<(), String> {
        let lookup_key = parent_task.to_lowercase();
        if let Some(task) = self.items.get_mut(&lookup_key) {
            let subtask_lookup = subtask_name.to_lowercase();
            if let Some(sub) = task.subtasks.iter_mut().find(|s| s.name.to_lowercase() == subtask_lookup) {
                sub.is_done = true;
                Ok(())
            } else {
                Err(format!("Subtask '{}' not found.", subtask_name))
            }
        } else {
            Err(format!("Parent task '{}' not found.", parent_task))
        }
    }

    // Helper method supporting standard formats like: "6:30 PM", "6:30pm", "18:30"
    fn parse_time_flexibly(time_str: &str) -> Option<NaiveTime> {
        let cleaned = time_str.trim().to_uppercase();
        
        NaiveTime::parse_from_str(&cleaned, "%I:%M %P")
            .or_else(|_| NaiveTime::parse_from_str(&cleaned, "%I:%M%P"))
            .or_else(|_| NaiveTime::parse_from_str(&cleaned, "%H:%M"))
            .ok()
    }

    fn print_list(&self) {
        if self.items.is_empty() {
            println!("No tasks found.");
            return;
        }

        let current_time = Local::now().time();

        for task in self.items.values() {
            let total = task.subtasks.len();
            let completed = task.subtasks.iter().filter(|s| s.is_done).count();
            
            let all_done = total > 0 && completed == total;
            
            // Check if deadline passed
            let mut missed_deadline = false;
            if let Some(ref end_str) = task.end_time {
                if let Some(deadline) = Self::parse_time_flexibly(end_str) {
                    if current_time > deadline {
                        missed_deadline = true;
                    }
                }
            }

            // Determine status badge
            let status = if all_done {
                "[DONE]"
            } else if missed_deadline {
                "[FAILED]"
            } else {
                "[TO DO]"
            };

            let schedule = match (&task.start_time, &task.end_time) {
                (Some(start), Some(end)) => format!(" [🕒 {} to {}]", start, end),
                (Some(start), None) => format!(" [🕒 Starts at {}]", start),
                (None, Some(end)) => format!(" [🕒 Ends by {}]", end),
                (None, None) => "".to_string(),
            };

            println!("{} {}{} ({}/{})", status, task.title, schedule, completed, total);
            for sub in &task.subtasks {
                let sub_status = if sub.is_done { "  ✓" } else { "  ☐" };
                println!("{} {}", sub_status, sub.name);
            }
            println!();
        }
    }
}

fn main() {
    let args = Cli::parse();
    const DB_FILE: &str = "todo.json";

    let mut todo = TodoList::load_from_file(DB_FILE);

    let result = match args.command.as_str() {
        "add" => match args.task {
            Some(task) => todo.add_task(task, args.start, args.end),
            None => Err("Task title cannot be empty!".to_string()),
        },
        "add-sub" => match (args.task, args.subtask) {
            (Some(task), Some(subtask)) => todo.add_subtask(&task, subtask),
            _ => Err("Usage: cargo run -- add-sub <task_title> <subtask_name>".to_string()),
        },
        "mark-done" => match (args.task, args.subtask) {
            (Some(task), Some(subtask)) => todo.mark_subtask_done(&task, &subtask),
            _ => Err("Usage: cargo run -- mark-done <task_title> <subtask_name>".to_string()),
        },
        "list" => {
            todo.print_list();
            Ok(())
        }
        cmd => Err(format!("Command {} not recognised", cmd)),
    };

    match result {
        Err(e) => println!("ERROR: {}", e),
        Ok(_) => {
            if let Err(e) = todo.save_to_file(DB_FILE) {
                println!("ERROR: Failed to save to disk: {}", e);
            } else {
                println!("SUCCESS");
            }
        }
    }
}
