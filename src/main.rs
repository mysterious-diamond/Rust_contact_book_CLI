use std::{io::{self, Write}, error::Error};

enum Token {
	Help(),
	Add(),
	Delete(),
	List(),
	Exit(),
	Args(String),
}

struct Contact {
	name: String,
	number: String,
}

impl Contact {
	pub fn new(name: String, number: String) -> Contact {
		Contact { name, number }
	}
}

fn help() {
	println!("Default commands :");
	println!("ls => Lists all contacts");
	println!();
	println!("add <args> => Adds a contact :");
	println!("\targs : name of contact, number");
	println!("\te.g. : add Cedric 067676767");
	println!();
	println!("del <arg> => Deletes a contact :");
	println!("\targs : name of contact");
	println!();
	println!("exit => Exits the program");
}

fn add(contacts: &mut Vec<Contact>, name: String, number: String) {
	let contact = Contact::new(name, number);
	contacts.push(contact);
	println!("Added contact to contact book");
}

fn list(contacts: &Vec<Contact>) {
	for contact in contacts {
		println!("{} : {}", contact.name, contact.number);
	}

	if contacts.is_empty() {
		println!("No contacts in contact book");
	}
}

fn delete(contacts: &mut Vec<Contact>, name: String) {
	contacts.retain(|contact| contact.name != name);
	println!("Removed contact {name} from contact book");	
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


fn parser(tokens: &[Token], contacts: &mut Vec<Contact>) -> Result<bool, Box<dyn Error>> {
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
			
			add(contacts, name, number);
		}
		else if let Token::Delete() = token {
			index += 1;
			let name: String;
			if index < tokens.len() && let Token::Args(arg) = &tokens[index] {
				name = arg.clone();
				delete(contacts, name);
				break;
			}
			
			println!("Expected argument after delete");
			break;
		}
		else if let Token::List() = token {
			list(contacts);
		}
		else if let Token::Exit() = token {
			return Ok(true);
		}

		index += 1;
	}
	
	Ok(false)
}

fn main() {
    println!("Contact book CLI interface, \nEnter \"help\" to see list of commands");
    let mut contacts: Vec<Contact> = Vec::new();
    
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
    	
    	let exit = parser(&tokens, &mut contacts).unwrap();
    	if exit {
    		break;
    	}
    }
}
