use rand::seq::SliceRandom;
use smallstr::SmallString;
use std::env::current_exe;

use crate::{Expression, VERSION};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
       SmallString::from("author") => Expression::String("Adam McDaniel, superiums".to_string()),
       SmallString::from("git") => Expression::String("https://codeberg.com/santo/lumesh".to_string()),
       SmallString::from("homepage") => Expression::String("https://codeberg.com/santo/lumesh".to_string()),
       SmallString::from("version") => Expression::String(VERSION.to_string()),
       SmallString::from("path") => {
            if let Ok(path) = current_exe() {
                Expression::String(path.to_str().unwrap().to_string())
            } else {
                Expression::None
            }
        },
       SmallString::from("suggestion") => {
            // Choose a random suggestion from the `help/suggestions.txt` file.
            let suggestions = include_str!("../../config/suggestions.txt");
            let suggestions = suggestions.split('\n').collect::<Vec<&str>>();
            let suggestion = suggestions.choose(&mut rand::thread_rng()).unwrap();
            Expression::String(suggestion.to_string())
        },
       SmallString::from("license") => Expression::String("APACHE-2.0".to_string()),
       SmallString::from("prelude") => {
            // Home directory + .lumesh-prelude


            if let Some(home_dir) = dirs::home_dir() {
                let prelude_path = home_dir.join(".lumesh-prelude");
                if prelude_path.exists() {
                    Expression::String(prelude_path.to_str().unwrap().to_string())
                } else {
                    Expression::None
                }
            } else {
                Expression::None
            }
        }
    })
    .into()
}
