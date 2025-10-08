use std::{io::{self, Write}, error::Error, env};
use mysql::*;
use mysql::prelude::*;
use dotenvy::dotenv;
use bcrypt::{DEFAULT_COST, hash, verify};
use colored::Colorize;

enum Token {
	Help(),
	Add(),
	Delete(),
	List(),
	Exit(),
	Args(String),
}

fn help() {
	println!("{}", "Default commands :".green().bold());
	println!("{}", "ls => Lists all contacts".green().bold());
	println!();
	println!("{}", "add <args> => Adds/Updates a contact :".green().bold());
	println!("\t{}", "args : name of new/existing contact, number".green().bold());
	println!("\t{}", "e.g. : add Cedric 067676767".green().bold());
	println!();
	println!("{}", "del <arg> => Deletes a contact :".green().bold());
	println!("\t{}", "args : name of contact".green().bold());
	println!();
	println!("{}", "exit => Exits the program".green().bold());
}

fn prompt(msg: &str) -> Result<String, Box<dyn std::error::Error>> {
	print!("{}", msg.yellow());
	io::stdout().flush()?;
	let mut input = String::new();
	io::stdin().read_line(&mut input)?;
	Ok(input)
}

fn add(name: String, number: String, conn: &mut PooledConn, user_id: u32) -> Result<(), Box<dyn std::error::Error>> {
	let query_result: Vec<u32> = conn.exec(
		"SELECT contact_id FROM contacts WHERE name = :name",
		params! {
			"name" => &name
		}
	)?;

	if !query_result.is_empty() {
		println!("{} {} {}", "Contact".red(), name.red(), "already exists, \nif you want to update a contact number consider using update".red().bold());
		return Ok(());
	}
	
	conn.exec_drop(
		"INSERT INTO contacts(user_id, name, number) VALUES (:user_id, :name, :number)",
		params! {
			"user_id" => user_id,
			"name" => name,
			"number" => number
		}
	)?;
	
	println!("{}", "Added contact to contact book".green().bold());
	Ok(())
}

fn list(conn: &mut PooledConn, user_id: u32) -> Result<(), Box<dyn std::error::Error>> {
	let query_result: Vec<(String, String)> = conn.exec(
		"SELECT name, number FROM contacts WHERE user_id = :user_id",
		params! {
			"user_id" => user_id
		}
	)?;

	if query_result.is_empty() {
		println!("{}", "No contacts in contact book".red().bold());
		return Ok(());
	}
	
	for contact in query_result {
		println!("{} : {}", contact.0.yellow(), contact.1.green());
	}

	Ok(())
}

fn delete(conn: &mut PooledConn, contact_name: String, user_id: u32) -> Result<(), Box<dyn std::error::Error>> {
	let query_result: Vec<u32> = conn.exec(
		"SELECT contact_id FROM contacts WHERE name = :name AND user_id = :user_id",
		params! {
			"name" => &contact_name,
			"user_id" => user_id
		}
	)?;

	if query_result.is_empty() {
		println!("{} {}", "No contact named".red().bold(), contact_name.red().bold());
		return Ok(());
	}

	conn.exec_drop(
		"DELETE FROM contacts WHERE contact_id = :contact_id",
		params! {
			"contact_id" => query_result[0]
		}
	)?;

	println!("{}", "Contact deleted successfully".green());

	Ok(())	
}

fn lexer(command: &str) -> Result<Vec<Token>, Box<dyn Error>> {
    let mut tokens = Vec::new();
    let mut index: usize = 0;
    let commands: Vec<char> = command.chars().collect();
    let mut curr_token = String::new();

    while index < commands.len() {
        if commands[index].is_alphanumeric() {
            curr_token.push(commands[index]);
            index += 1;
            continue;
        }

		
        if commands[index] == '"' {
        	index += 1;
        	while commands[index] != '"' {
        		curr_token += commands[index].to_string().as_str();
        		index += 1;
        	}
        	index += 1;
        	continue;
        }

        if commands[index] == ' ' || commands[index] == '\n' {
            if !curr_token.is_empty() {
                tokens.push(match curr_token.as_str() {
                    "help" => Token::Help(),
                    "add" => Token::Add(),
                    "del" => Token::Delete(),
                    "ls" => Token::List(),
                    "exit" => Token::Exit(),
                    arg => Token::Args(arg.to_string()),
                });
                curr_token.clear();
            }
            index += 1;
            continue;
        }

        index += 1;
    }

    if !curr_token.is_empty() {
        tokens.push(match curr_token.as_str() {
            "help" => Token::Help(),
            "add" => Token::Add(),
            "del" => Token::Delete(),
            "ls" => Token::List(),
            "exit" => Token::Exit(),
            arg => Token::Args(arg.to_string()),
        });
    }

    Ok(tokens)
}


fn parser(tokens: &[Token], conn: &mut PooledConn, user_id: u32) -> Result<bool, Box<dyn Error>> {
	let mut index: usize = 0;
	while index < tokens.len() {
		let token = &tokens[index];
		if let Token::Help() = token {
			help();
		}
		else if let Token::Add() = token {
			index += 1;
			let name: String;
			if index < tokens.len() && let Token::Args(arg) = &tokens[index] {
				name = arg.clone();	
			}
			else {
				println!("{}", "Expected arguments after add".red());
				break;
			}

			index += 1;
			let number: String;
			if index < tokens.len() && let Token::Args(arg) = &tokens[index] {
				number = arg.clone();	
			}
			else {
				println!("{}", "Expected 2 arguments after add".red());
				break;
			}
			
			add(name, number, conn, user_id)?;
		}
		else if let Token::Delete() = token {
			index += 1;
			let name: String;
			if index < tokens.len() && let Token::Args(arg) = &tokens[index] {
				name = arg.clone();
				delete(conn, name, user_id)?;
				break;
			}
			
			println!("{}", "Expected argument after delete".red());
			break;
		}
		else if let Token::List() = token {
			list(conn, user_id)?;
		}
		else if let Token::Exit() = token {
			return Ok(true);
		}

		index += 1;
	}
	
	Ok(false)
}

fn login(conn: &mut PooledConn) -> Result<u32, Box<dyn std::error::Error>> {
	let username = prompt("Please login first, enter your username : ")?.trim().to_string();
	let password = prompt("Password : ")?.trim().to_string();

	let query_result: Vec<(u32, String)> = conn.exec(
		"SELECT user_id, password FROM users WHERE username = :username",
		params! {
			"username" => username
		}
	)?;

	if query_result.is_empty() || !verify(password, query_result[0].1.as_str())? {
		return Err("Username/Password not valid".into());
	}

	Ok(query_result[0].0)
}

fn signup(conn: &mut PooledConn) -> Result<u32, Box<dyn std::error::Error>> {
	let mut user_id: u32 = 0;
	let mut username = String::new();
	for i in 1..=3 {
		username = prompt("Enter a username : ")?.trim().to_string();

		let query_result: Vec<u32> = conn.exec(
			"SELECT user_id FROM users WHERE username = :username",
			params! {
				"username" => &username
			}
		)?;

		if query_result.is_empty() {
			break;
		}

		if i != 3 {
			println!("{}", "Username already in use, please try again".red());
		}
		else {
			return Err("Unable to signup, 3 invalid signup attempts".into());
		}
	}

	let password = hash(prompt("Enter a password : ")?.trim(), DEFAULT_COST)?;

	conn.exec_drop(
		"INSERT INTO users(username, password) VALUES (:username, :password)",
		params! {
			"username" => &username,
			"password" => &password
		}
	)?;

	user_id = conn.exec(
		"SELECT user_id FROM users WHERE username = :username",
		params! {
			"username" => username
		}
	)?[0];

	Ok(user_id)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Contact book CLI interface".green());

    dotenv().ok();
    
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let pool = Pool::new(url.as_str())?;
	let mut conn = pool.get_conn()?;

	let choice: u8 = prompt("Hello, enter 1 to login, anything else to sign up.")?.trim().parse()?;

	let mut user_id: u32 = 0;
	if choice == 1 {
		user_id = match login(&mut conn) {
			Ok(id) => {
				println!("{}", "Login succesful.".green());
				id
			},
			Err(msg) => panic!("{} {}", "Login unsuccesful,".red(), msg.to_string().red())
		};
	}
	else {
		user_id = match signup(&mut conn) {
			Ok(id) => {
				println!("{}", "Signup succesful, account has been created.".green());
				id
			},
			Err(msg) => panic!("{} {}", "Signup unsuccesful,".red(), msg.to_string().red())
		};
	}

	println!();
	println!("{}", "Enter \"help\" for list of commands".green().bold());
        
    loop {
    	let command = prompt(">> ")?.trim().to_string();
    	
    	let tokens = match lexer(command.as_str()) {
    		Ok(tokens) => tokens,
    	   	Err(err) => panic!("{} {err}", "Wasn't able to lex command, ".red())
    	};
    	
    	let exit = match parser(&tokens, &mut conn, user_id) {
    		Ok(val) => val,
    		Err(msg) => panic!("{} {msg}", "Wasn't able to execute command,".red())
    	};
    	if exit {
    		break;
    	}
    }

    Ok(())
}
