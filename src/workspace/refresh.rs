use std::collections::HashMap;

use regex::Regex;

use crate::{model::{Configuration, Project}, util::consts::print_action_header, workspace::nature::Nature};

pub(crate) fn refresh(project: &Project, configuration: &Configuration, regexes: &HashMap<&str, Regex>) -> Result<(), (Nature, String)>
{
    print_action_header("Removing natures", 1, 2);
    for nature in Nature::values() {
        print!("> Removing project nature \"{}\" ... ", nature.type_str());
        nature.remove_nature()
            .map_err(| e | (nature, e))?;

        println!("Done!");
    }
    println!("Done!");

    print_action_header("Applying natures", 2, 2);
    for nature in project.info().natures() {
        print!("> Applying project nature \"{}\"... ", nature.type_str());
        if let Err(e) = nature.setup_nature(&project, configuration, &regexes)
        {
            println!("Failed: {e}");
            print!("Deleting project nature \"{}\" for project cleanliness ... ", nature.type_str());
            
            match nature.remove_nature() {
                Ok(_) => println!("Done."),
                Err(e) => {
                    println!("Failed: {e}, you may have to clean the project manually.")
                },
            }
            return Err((nature.clone(), e))
        }
        println!("Done!");
    }

    Ok(())
}
