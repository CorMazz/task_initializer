//! A command line tool to copy a template directory layout, currently hardcoded to be within C:\Users\USERNAME\.task_initializer, to the specified location.

use path_clean::PathClean;
use std::path::{PathBuf, Path};
use std::{env, fs};
use std::io::{self, Write};
use std::fs::read_dir;
use clap::Parser;
use regex::Regex;
use whoami;
/// Create the task initializer command line interface and run the appropriate methods.
fn main() {
    // Parse the command line arguments
    let args = Args::parse();

    if args.debug {
        // You can now access the parsed arguments as fields of the `args` struct.
        println!("Task Name: {:?}", absolute_path(args.task_name.clone()).unwrap());
        println!("Layout: {}", args.layout);
        println!("Numbering: {}", args.numbering);
        println!("Renumber: {}", args.renumber);
        println!("Force Renumber?: {}", args.force);
    }

    // Feed the command line arguments to the task initializer
    let task_initializer = TaskInitializer::new(args);
    task_initializer.run();
}

//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------
// Task Initializer
//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct TaskInitializer {
    // Define a pathbuf to store the base directory of the layout options
    layout_repository_dir: PathBuf,
    // Define an Args to store the parsed command line args
    args: Args
}

impl TaskInitializer {


//-----------------------------------------------------------------------------------------------------------------------
// Initialization Method
//-----------------------------------------------------------------------------------------------------------------------

    ///Initialization Method
    pub fn new(args: Args) -> Self {
        // Grab the person's user name
        let username = whoami::username();


        // Initialize the layout repository dir PathBuf with the layout options directory
        let mut layout_repository_dir = PathBuf::new();
        layout_repository_dir.push(r"C:\Users");
        layout_repository_dir.push(username);
        layout_repository_dir.push(r".task_initializer");

        if args.debug {
            println!("Looking for layouts in: {:?}", layout_repository_dir)
        }

        // Check if the path exists
        if !layout_repository_dir.exists() {
            // If the path doesn't exist, create the directory
            if let Err(err) = fs::create_dir_all(&layout_repository_dir) {
                // Handle the error, printing a message for simplicity
                println!("Error creating the layout repository directory, something weird is going on: {}", err);
                std::process::exit(1); // Exit with a non-zero code indicating an error
            } else {
                // Print a message and exit the program
                println!("This must be your first time running. Created the layout repository directory. Put some layouts in here and run this again\n{:?}", layout_repository_dir);
                std::process::exit(0); // Exit with a zero code indicating success
            }
        }

        // Create and return a TaskInitializer instance
        TaskInitializer { 
            layout_repository_dir,
            args: args,
        }
    }


//-----------------------------------------------------------------------------------------------------------------------
// Main Run Method
//-----------------------------------------------------------------------------------------------------------------------

    /// The main run method for the class calls the correct sub-method for the given args. 
    fn run(&self) {

        // Check to see if renumber mode is activated:
        if self.args.renumber {
            if let Err(err) = self.renumber_directory() {
                // Handle the error returned if there is an error
                eprintln!("Error during directory renumbering: {:?}", err);
            }
        }
        // Otherwise copy the template directory over.
        else {
            if let Err(err) = self.copy_templates() {
                // Handle the error returned if there is an error
                eprintln!("Error during task creation: {:?}", err);
            }
        }

    }

//-----------------------------------------------------------------------------------------------------------------------
// Copy Templates Method
//-----------------------------------------------------------------------------------------------------------------------

    /// Copy the template directory over to the desired task name directory.
    fn copy_templates(&self) -> Result<(), CustomError> {
        // Read the layout directories in
        let layout_paths = read_dir(&self.layout_repository_dir).unwrap();

        let layout_names: Vec<String> = layout_paths
            .filter_map( |entry| {
                entry
                    .ok() // Convert Result<DirEntry, io::Error> to Option<DirEntry>
                    .map(|dir_entry| dir_entry.file_name()) // Extract the OsString file name
                    .and_then(|os_str| os_str.into_string().ok()) // Convert OsString to String
                }
            )
            .collect();

        // Check to see if self.args.layout is contained within the layout_names
        if !layout_names.contains(&self.args.layout) {
            return Err(CustomError::LayoutNotFound { layout: self.args.layout.clone(), available_layouts: layout_names })
        };

        // Create the path to the full layout 
        let layout_dir: PathBuf = self.layout_repository_dir.join(&self.args.layout);

        // Split off the just the base name of the directory from the tast_name path
        let task_parent_dir = self.args.task_name.parent().unwrap();
        let mut task_name = self.args.task_name.file_name().and_then(|name| name.to_str()).unwrap();
        
        // Create the start of the full task_path
        let mut task_path: PathBuf = task_parent_dir.to_path_buf();

        // If we want to automagically add/fix the numbering
        if self.args.numbering {
            // Check to see if numbering was already specified
            if let Some(strip_starting_digits) = Regex::new(r"^\d+").unwrap().find(&task_name) {
                task_name = task_name
                    .strip_prefix(strip_starting_digits.as_str())
                    .unwrap();
            }

            // Strip off a leading _ if it begins with one (used to allow numbers in the beginning of names, such as 9F_evo specified via _9F_evo)
            task_name = task_name.trim_start_matches("_");

            // Create a regex to find 3 digits in the beginning of a line
            let three_digits_regex = Regex::new(r"^\d{3}").unwrap();

            // Now search the directory and find all other task directories
            let mut existing_tasks: Vec<String> = read_dir(task_parent_dir)
                .unwrap()
                .filter_map(|entry| entry.ok()) // Filter out any errors within the lazily constructed readdir obj

                // Get just directories
                .filter(|entry| {
                    entry
                        .file_type() // Get the file type of the entry
                        .map(|file_type| file_type.is_dir()) // Check if its a directory
                        .unwrap_or(false) // If there was an error getting the file type, consider it as not a directory
                })

                // Now get just the basenames of the tasks, not the whole path
                .filter_map(|task| { 
                    task
                        .path() // Get the path obj from the DirEntry
                        .file_name() // Get just the file name
                        .and_then(|name| name.to_str()) // Turn it into an &str
                        .map(|name| name.to_string()) // Turn it into a real String
                    }   
                )

                // Now get just the task names that start with 3 digits
                .filter(|task| three_digits_regex.find(&task).is_some())
                .collect();

            // Sort them in ascending order
            existing_tasks.sort();
            
            // Figure out the next task number in the sequence of existing tasks

            // Declare the variable so that it lives in this scope
            let next_task_number_str: String;
            if let Some(last_task) = existing_tasks.last() {
                // Get the last task number
                let last_task_number = three_digits_regex.find(last_task).unwrap().as_str().parse::<u32>().unwrap();
                let next_task_number = last_task_number + 1;
                next_task_number_str = format!("{:03}", next_task_number); // Turn it into a string
            } else {
                next_task_number_str = "000".to_string(); // If no existing tasks, start with "000"
            } 

            // Combine the next task number with the task_name
            let new_task_name = format!("{}_{}", next_task_number_str, task_name);
            
            task_path.push(&new_task_name);
        } else {
            // Just add the task name to the task path
            task_path.push(task_name);
        }

        copy_tree(layout_dir, task_path.clone()).expect("Error during copying");
        println!("Successfully created {:?}", absolute_path(&task_path).unwrap());
        
        // Add the project initialization date to the readme.docx file. 
        


        Ok(())
    }

//-----------------------------------------------------------------------------------------------------------------------
// Renumber Directory Method
//-----------------------------------------------------------------------------------------------------------------------

    fn renumber_directory(&self) -> io::Result<()> {
    
        // Create a regex to find 3 digits in the beginning of a line
        let three_digits_regex = Regex::new(r"^\d{3}").unwrap();

        // Get all subdirectories within the directory
        let mut existing_tasks: Vec<String> = read_dir(&self.args.task_name)
            .expect("Error while reading the parent directory")
            .filter_map(|entry| entry.ok()) // This gets rid of the busted directories

            // Now lets get just directory names
            .filter(|entry| {
                entry   
                    .file_type()
                    .map(|file_type| file_type.is_dir())
                    .unwrap_or(false)
                }
            )

            // Now lets grab just the basenames of the directories, not the whole paths
            .filter_map(|task| {
                task
                    .path() // grab the path obj
                    .file_name() // grab the file name from it
                    .and_then(|name| name.to_str()) // turn it into an &str
                    .map(|name| name.to_string()) // turn it into a real String
                }
            )

            // Lets grab just the ones that start with 3 numbers
            .filter(|task| three_digits_regex.find(&task).is_some())
            .collect();
            
            // Define a regex that will match a three digit integer or a variable precision float at the beginning of a string
            let sort_parse_regex: Regex = Regex::new(r"^(\d{3})(\.\d*)?").unwrap();

            // Sort the existing tasks by their numbers in the beginning
            existing_tasks.sort_by(|a, b| {
                sort_parse_regex.find(a).unwrap().as_str().parse::<f64>().unwrap()
                    .partial_cmp(&sort_parse_regex.find(b).unwrap().as_str().parse::<f64>().unwrap())
                    .unwrap()
                }
            );

            // Strip off the original numbering from the task
            let renumbered_tasks: Vec<String> = existing_tasks
            .iter() // Turn it into an iterator
            
            // Chop off the prefix that we locate with the regex
            
            .map(|task| { 
                task
                    .strip_prefix(
                        sort_parse_regex
                            .find(task)
                            .unwrap()
                            .as_str()
                    )
                    .unwrap()
                    .to_string()
                }
            )

            // add a new prefix back on, our desired numbering

            .enumerate() 
            .map(|(i, task)| {
                format!("{:03}{}", i, &task)
                }
            )
            .collect();

        // Prompt user for confirmation

        let mut input: String;
        if !self.args.force {
            println!("This will rename the contents of the folder {:?}", absolute_path(&self.args.task_name).unwrap());
            print!("Do you want to proceed with the renaming? (y/n): ");
            io::stdout().flush().unwrap();

            input = String::new();
            io::stdin().read_line(&mut input)?;
        } else {
            input = String::from("y");
        }

        if input.trim().eq_ignore_ascii_case("y") {
            // User confirmed, proceed with renaming
            for (old_name, new_name) in existing_tasks.iter().zip(&renumbered_tasks) {
                fs::rename(&self.args.task_name.join(old_name), &self.args.task_name.join(new_name))?;
            }

            println!("Renaming completed successfully.");
            Ok(())
        } else {
            println!("Renaming canceled by user.");
            Ok(())
        }
        

    }


}

//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------
// Args
//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------

/// Creates a task folder structure based on a template layout. Can also renumber tasks appropriately.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The directory name where the task folder will be created. Can be a whole path or relative to the cwd. If the renumber
    /// flag is specified, this is the parent directory where all sub-directories will be renamed.
    #[arg()]
    task_name: PathBuf,

    /// The directory layout to use. Layout options are found in your C:\Users\USERNAME\.task_initializer folder.
    #[arg(short, long, default_value = "default")]
    layout: String,

    /// Specify if you DO NOT want to add numbering to the beginning of the folder path.
    /// Otherwise, will remove existing numbers at the beginning of a path and add new ones.
    #[arg(short, long, action = clap::ArgAction::SetFalse)]
    numbering: bool,

    /// Parse the given directory and renumber all tasks so that numbering is contiguous.
    /// Keeps the original task order. To insert a task in between two other tasks, use decimal numbers.
    /// ie. Task 001.5 would be renumbered after task 001. Tasks with the same number will be sorted alphabetically.
    #[arg(short, long, action)]
    renumber: bool,

    /// Set all user prompts to yes and proceed with the task. Only affects the renumber method which would normally prompt the user for confirmation.
    #[arg(short, long, action)]
    force: bool,

    /// Run the tool in debug mode and print various outputs.
    #[arg(short, long, action)]
    debug: bool,
}

//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------
// Functions
//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------

//-----------------------------------------------------------------------------------------------------------------------
// Absolute Path
//-----------------------------------------------------------------------------------------------------------------------

/// Get the absolute path from a path
/// <https://stackoverflow.com/questions/30511331/getting-the-absolute-path-from-a-pathbuf>
fn absolute_path(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }.clean();

    Ok(absolute_path)
}

//-----------------------------------------------------------------------------------------------------------------------
// Copy Tree
//-----------------------------------------------------------------------------------------------------------------------

/// Copy a directory tree recursively
/// <https://stackoverflow.com/questions/26958489/how-to-copy-a-folder-recursively-in-rust>
fn copy_tree(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let dst_path = dst.as_ref();
    
    if dst_path.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Destination directory already exists"));
    }
    
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_tree(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------
// Enums
//-----------------------------------------------------------------------------------------------------------------------
//-----------------------------------------------------------------------------------------------------------------------

//-----------------------------------------------------------------------------------------------------------------------
// Custom Errors
//-----------------------------------------------------------------------------------------------------------------------

/// An enum to display a custom "LayoutNotFound" error.
#[derive(Debug)]
#[allow(dead_code)]
enum CustomError {
    LayoutNotFound { layout: String, available_layouts: Vec<String> }
}
