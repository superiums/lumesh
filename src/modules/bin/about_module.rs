use rand::seq::IndexedRandom;
use std::env::current_exe;

use crate::{Expression, VERSION};
use common_macros::hash_map;

pub fn get() -> Expression {
    (hash_map! {
        String::from("author") => Expression::String("Santo, Adam McDaniel".to_string()),
        String::from("git") => Expression::String("https://codeberg.com/santo/lumesh".to_string()),
        String::from("homepage") => Expression::String("https://lumesh.codeberg.page".to_string()),
        String::from("version") => Expression::String(VERSION.to_string()),
        String::from("bin") => {
            if let Ok(path) = current_exe() {
                Expression::String(path.to_str().unwrap().to_string())
            } else {
                Expression::None
            }
        },
        String::from("tip") => {
            // Choose a random suggestion from the `help/suggestions.txt` file.
            let suggestions = include_str!("../../config/suggestions.txt");
            let suggestions = suggestions.split('\n').collect::<Vec<&str>>();
            let suggestion = suggestions.choose(&mut rand::rng()).unwrap();
            Expression::String(suggestion.to_string())
        },
        String::from("license") => Expression::String("MIT".to_string()),
        String::from("prelude") => {
            if let Some(c) = dirs::config_dir() {
                let prelude_path = c.join("lumesh/config.lm");
                if prelude_path.exists() {
                    Expression::String(prelude_path.to_str().unwrap().to_string())
                } else {
                    Expression::String(prelude_path.to_str().unwrap().to_string()+" !")
                }
            } else {
                Expression::String("config.lm".to_string())
            }
        }
    })
    .into()
}
