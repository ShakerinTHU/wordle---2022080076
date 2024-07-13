use clap::{Arg, Command};
use console::style;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;

mod builtin_words;
use builtin_words::{FINAL, ACCEPTABLE};

#[derive(Serialize, Deserialize)]
struct GameState {
    total_rounds: usize,
    successful_games: usize,
    attempts: usize,
    used_words: HashMap<String, usize>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            total_rounds: 0,
            successful_games: 0,
            attempts: 0,
            used_words: HashMap::new(),
        }
    }

    fn load(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let state: GameState = serde_json::from_str(&contents)?;
        Ok(state)
    }

    fn save(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(file_path, contents)?;
        Ok(())
    }

    fn add_word(&mut self, word: &str) {
        *self.used_words.entry(word.to_string()).or_insert(0) += 1;
    }

    fn print_stats(&self) {
        println!("Games played: {}", self.total_rounds);
        println!("Successful games: {}", self.successful_games);
        println!(
            "Average attempts: {:.2}",
            self.attempts as f64 / self.successful_games as f64
        );
        let mut sorted_words: Vec<_> = self.used_words.iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        println!("Most frequently used words:");
        for (word, count) in sorted_words.iter().take(5) {
            println!("{}: {}", word, count);
        }
    }
}

/// The main function for the Wordle game
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Command-line argument parsing
    let matches = Command::new("Wordle")
        .version("1.0")
        .about("Wordle Game in Rust")
        .arg(
            Arg::new("word")
                .short('w')
                .long("word")
                .num_args(1)
                .help("Specify the answer word"),
        )
        .arg(
            Arg::new("random")
                .short('r')
                .long("random")
                .num_args(0)
                .help("Enable random mode"),
        )
        .arg(
            Arg::new("seed")
                .short('s')
                .long("seed")
                .num_args(1)
                .requires("random")
                .help("Specify the seed for random word generation"),
        )
        .arg(
            Arg::new("difficult")
                .short('d')
                .long("difficult")
                .num_args(0)
                .help("Enable hard mode"),
        )
        .arg(
            Arg::new("state")
                .short('S')
                .long("state")
                .num_args(1)
                .help("Specify the state file for saving/loading game state"),
        )
        .get_matches();

    let is_tty = atty::is(atty::Stream::Stdout);

    if is_tty {
        println!(
            "I am in a tty. Please print {}!",
            style("colorful characters").bold().blink().blue()
        );
    } else {
        println!("I am not in a tty. Please print according to test requirements!");
    }

    if is_tty {
        print!("{}", style("Your name: ").bold().red());
        io::stdout().flush().unwrap();
    }
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let name = line.trim().to_string();
    println!("Debug: Welcome to wordle, {}!", name);

    // Prepare the ACCEPTABLE list for case-insensitive comparison
    let acceptable_set: HashSet<String> = ACCEPTABLE.iter().map(|&word| word.to_uppercase()).collect();
    let final_set: HashSet<String> = FINAL.iter().map(|&word| word.to_uppercase()).collect();

    // Debug: Print the lengths of the lists
    println!("Debug: ACCEPTABLE list length: {}", acceptable_set.len());
    println!("Debug: FINAL list length: {}", final_set.len());

    // Load game state if specified
    let state_file = matches.get_one::<String>("state").map(String::as_str).unwrap_or("game_state.json");
    let mut game_state = GameState::load(state_file).unwrap_or_else(|_| GameState::new());

    loop {
        game_state.total_rounds += 1;

        // Determine the answer word
        let answer = if matches.contains_id("random") {
            // Random mode
            println!("Debug: Random mode enabled");
            let seed = matches.get_one::<String>("seed")
                .map_or_else(|| {
                    let seed = rand::thread_rng().gen::<u64>();
                    println!("Debug: Generated random seed: {}", seed);
                    seed
                }, |s| {
                    let seed = s.parse().unwrap_or_else(|_| {
                        println!("Debug: Invalid seed provided, using default random seed");
                        rand::thread_rng().gen::<u64>()
                    });
                    seed
                });
            println!("Debug: Seed used: {}", seed);
            let mut rng = StdRng::seed_from_u64(seed);
            let random_word = FINAL.choose(&mut rng).unwrap().to_string().to_uppercase();
            println!("Debug: Random word selected: {}", random_word);
            random_word
        } else if let Some(word) = matches.get_one::<String>("word") {
            // Specified answer word
            println!("Debug: Specified word mode enabled");
            word.to_string().to_uppercase()
        } else {
            // Prompt for input if not provided
            println!("Debug: Manual mode enabled");
            println!("Please enter the answer word (5 letters):");
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            answer.trim().to_uppercase()
        };

        // Validate the answer word
        println!("Debug: Checking if the answer is in the ACCEPTABLE list");
        if !acceptable_set.contains(&answer) {
            println!("The answer word must be a valid 5-letter word from the word list.");
            game_state.total_rounds -= 1;
            continue;
        }

        // Debug: Show the entered answer
        println!("Debug: Answer entered: {}", answer);

        // Prompt for difficulty selection
        println!("Do you want to enable hard mode? (y/n)");
        let mut difficulty_response = String::new();
        io::stdin().read_line(&mut difficulty_response)?;
        let hard_mode = difficulty_response.trim().to_lowercase() == "y";

        // Game loop
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 6;

        // Debug: Indicate entering the game loop
        println!("Debug: Entering game loop");

        let mut correct_positions = vec!['_'; 5];
        let mut present_letters = HashSet::new();

        while attempts < MAX_ATTEMPTS {
            println!("Attempt {}: Enter your guess:", attempts + 1);
            let mut guess = String::new();
            io::stdin().read_line(&mut guess)?;
            let guess = guess.trim().to_uppercase();

            // Debug: Show the entered guess
            println!("Debug: Guess entered: {}", guess);

            if !acceptable_set.contains(&guess) {
                println!("Invalid guess. Please enter a 5-letter word from the word list.");
                continue;
            }

            // Validate hard mode guess
            if hard_mode && !is_valid_hard_mode_guess(&guess, &correct_positions, &present_letters) {
                println!("Invalid guess in hard mode. Correct positions and present letters must be maintained.");
                continue;
            }

            // Provide feedback for the guess
            let feedback = provide_feedback(&guess, &answer, is_tty);
            println!("Feedback: {}", feedback);

            attempts += 1;
            game_state.attempts += 1;
            game_state.add_word(&guess);

            if guess == answer {
                println!("Congratulations! You've guessed the word.");
                game_state.successful_games += 1;
                break;
            }

            // Update correct positions and present letters for hard mode
            if hard_mode {
                for (i, c) in guess.chars().enumerate() {
                    if answer.chars().nth(i) == Some(c) {
                        correct_positions[i] = c;
                    } else if answer.contains(c) {
                        present_letters.insert(c);
                    }
                }
            }

            if attempts == MAX_ATTEMPTS {
                println!("Sorry, you've used all attempts. The word was '{}'.", answer);
            }
        }

        println!("Do you want to play another round? (y/n)");
        let mut play_again = String::new();
        io::stdin().read_line(&mut play_again)?;
        if play_again.trim().to_lowercase() != "y" {
            break;
        }
    }

    // Print statistics
    game_state.print_stats();

    // Save game state
    game_state.save(state_file)?;

    Ok(())
}

fn is_valid_hard_mode_guess(guess: &str, correct_positions: &[char], present_letters: &HashSet<char>) -> bool {
    for (i, c) in guess.chars().enumerate() {
        if correct_positions[i] != '_' && correct_positions[i] != c {
            return false;
        }
    }

    for &c in present_letters {
        if !guess.contains(c) {
            return false;
        }
    }

    true
}

fn provide_feedback(guess: &str, answer: &str, is_tty: bool) -> String {
    let mut feedback = String::new();
    let mut answer_chars: Vec<char> = answer.chars().collect();
    let guess_chars: Vec<char> = guess.chars().collect();

    // First pass: Check for correct positions (green)
    for (i, c) in guess_chars.iter().enumerate() {
        if answer_chars[i] == *c {
            if is_tty {
                feedback.push_str(&format!("{}", style(c).green()));
            } else {
                feedback.push('G');
            }
            answer_chars[i] = '_'; // Mark this character as matched
        } else {
            feedback.push('_'); // Placeholder for second pass
        }
    }

    // Second pass: Check for correct letters in wrong positions (yellow)
    for (i, c) in guess_chars.iter().enumerate() {
        if feedback.chars().nth(i) == Some('_') {
            if answer_chars.contains(c) {
                if is_tty {
                    feedback.replace_range(i..=i, &format!("{}", style(c).yellow()));
                } else {
                    feedback.replace_range(i..=i, "Y");
                }
                let pos = answer_chars.iter().position(|&x| x == *c).unwrap();
                answer_chars[pos] = '_'; // Mark this character as matched
            } else {
                if is_tty {
                    feedback.replace_range(i..=i, &format!("{}", style(c).red()));
                } else {
                    feedback.replace_range(i..=i, "R");
                }
            }
        }
    }

    feedback
}
