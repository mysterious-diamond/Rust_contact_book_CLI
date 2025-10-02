use std::{io::{self, Write}, error::Error, env};
use mysql::*;
use mysql::prelude::*;
use dotenvy::dotenv;

enum Token {
	Help(),
	Add(),
	Delete(),
	List(),
	Exit(),
	Args(String),
}

fn help() {
	println!("Default commands :");
	println!("ls => Lists all contacts");
	println!();
	println!("add <args> => Adds/Updates a contact :");
	println!("\targs : name of new/existing contact, number");
	println!("\te.g. : add Cedric 067676767");
	println!();
	println!("del <arg> => Deletes a contact :");
	println!("\targs : name of contact");
	println!();
	println!("exit => Exits the program");
}

fn add(name: String, number: String, conn: &mut PooledConn, user_id: u32) -> Result<(), Box<dyn std::error::Error>> {
	conn.exec_drop(
		"INSERT INTO contacts(user_id, name, number) VALUES (:user_id, :name, :number)",
		params! {
			"user_id" => user_id,
			"name" => name,
			"number" => number
		}
	)?;
	
	println!("Added contact to contact book");
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
		println!("No contacts in contact book");
		return Ok(());
	}
	
	for contact in query_result {
		println!("{} : {}", contact.0, contact.1);
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
		println!("No contact named {contact_name}");
		return Ok(());
	}

	conn.exec_drop(
		"DELETE FROM contacts WHERE contact_id = :contact_id",
		params! {
			"contact_id" => query_result[0]
		}
	)?;

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
				println!("Expected arguments after add");
				break;
			}

			index += 1;
			let number: String;
			if index < tokens.len() && let Token::Args(arg) = &tokens[index] {
				number = arg.clone();	
			}
			else {
				println!("Expected 2 arguments after add");
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
			
			println!("Expected argument after delete");
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

fn login(conn: &mut PooledConn) -> u32 {
	print!("Please login first, enter your username : ");
	io::stdout().flush().unwrap();
	let mut username = String::new();
	io::stdin().read_line(&mut username).unwrap();
	let username = username.trim();
	
	print!("Password : ");
	io::stdout().flush().unwrap();
	let mut password = String::new();
	io::stdin().read_line(&mut password).unwrap();
	let password = password.trim();

	let query_result: Vec<u32> = conn.exec(
		"SELECT user_id FROM users WHERE username = :username AND password = :password",
		params! {
			"username" => username,
			"password" => password
		}
	).unwrap();

	if query_result.is_empty() {
		panic!("User not valid");
	}

	query_result[0]
}

fn signup(conn: &mut PooledConn) -> Result<u32, Box<dyn std::error::Error>> {
	let mut user_id: u32 = 0;
	let mut username = String::new();
	for i in 1..=3 {
		print!("Enter a username : ");
		io::stdout().flush()?;
		let mut input = String::new();
		io::stdin().read_line(&mut input)?;
		username = input.trim().to_string();

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
			println!("Username already in use, please try again");
		}
		else {
			return Err("Unable to signup, 3 invalid signup attempts".into());
		}
	}

	print!("Enter a password : ");
	io::stdout().flush()?;
	let mut password = String::new();
	io::stdin().read_line(&mut password)?;
	let password = password.trim();

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
    println!("Contact book CLI interface");

    dotenv().ok();
    
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let pool = Pool::new(url.as_str())?;
	let mut conn = pool.get_conn()?;

	print!("Hello, enter 1 to login, anything else to sign up.");
	io::stdout().flush().unwrap();
	let mut choice = String::new();
	io::stdin().read_line(&mut choice).unwrap();
	let choice: u8 = choice.trim().parse()?;

	let mut user_id: u32 = 0;
	if choice == 1 {
		user_id = login(&mut conn);
	}
	else {
		user_id = signup(&mut conn)?;
	}
        
    loop {
    	print!(">> ");
    	io::stdout().flush().unwrap();
    	
    	let mut command = String::new();
    	io::stdin().read_line(&mut command).unwrap();
    	command = command.trim().to_string();
    	
    	let tokens = match lexer(command.as_str()) {
    		Ok(tokens) => tokens,
    	   	Err(err) => panic!("Wasn't able to lex command, {err}")
    	};
    	
    	let exit = parser(&tokens, &mut conn, user_id).unwrap();
    	if exit {
    		break;
    	}
    }

    Ok(())
}
