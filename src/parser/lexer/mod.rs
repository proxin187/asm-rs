use std::io::BufReader;
use std::io::BufRead;
use std::io::Seek;
use std::fs::File;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Keyword {
    Syscall,

    Je,
    Jg,
    Jb,
    Jmp,
    Cmp,

    Mov,
    Add,

    Pop,
    Push,

}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Register {
    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    Esp,
    Ebp,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Symbol {
    Colon,
    Comma,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Token {
    Register(Register),
    Keyword(Keyword),
    Symbol(Symbol),
    Ident(String),
    Int(i32),
    Eof,
}

pub struct Lexer {
    reader: BufReader<File>,
}

impl Lexer {
    pub fn new(file: &str) -> Result<Lexer, Box<dyn std::error::Error>> {
        let fd = File::open(file)?;

        Ok(Lexer {
            reader: BufReader::new(fd),
        })
    }

    fn lex_token(&mut self, token: &str) -> Result<Token, Box<dyn std::error::Error>> {
        match token.to_lowercase().as_str() {
            "push" => Ok(Token::Keyword(Keyword::Push)),
            "pop" => Ok(Token::Keyword(Keyword::Pop)),

            "mov" => Ok(Token::Keyword(Keyword::Mov)),
            "add" => Ok(Token::Keyword(Keyword::Add)),

            "cmp" => Ok(Token::Keyword(Keyword::Cmp)),
            "jmp" => Ok(Token::Keyword(Keyword::Jmp)),
            "je" => Ok(Token::Keyword(Keyword::Je)),
            "jg" => Ok(Token::Keyword(Keyword::Jg)),
            "jb" => Ok(Token::Keyword(Keyword::Jb)),

            "syscall" => Ok(Token::Keyword(Keyword::Syscall)),

            "eax" => Ok(Token::Register(Register::Eax)),
            "ebx" => Ok(Token::Register(Register::Ebx)),
            "ecx" => Ok(Token::Register(Register::Ecx)),
            "edx" => Ok(Token::Register(Register::Edx)),
            "esi" => Ok(Token::Register(Register::Esi)),
            "edi" => Ok(Token::Register(Register::Edi)),
            "esp" => Ok(Token::Register(Register::Esp)),
            "ebp" => Ok(Token::Register(Register::Ebp)),

            ":" => Ok(Token::Symbol(Symbol::Colon)),
            "," => Ok(Token::Symbol(Symbol::Comma)),
            _ => {
                if let Ok(integer) = token.parse::<i32>() {
                    Ok(Token::Int(integer))
                } else {
                    Ok(Token::Ident(token.to_string()))
                }
            },
        }
    }

    fn lex_line(&mut self, line: &str) -> Result<Vec<Token>, Box<dyn std::error::Error>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut token = String::new();

        for character in line.chars() {
            if [' ', ',', ':', '\n'].contains(&character) {
                if !token.is_empty() {
                    tokens.push(self.lex_token(&token)?);
                }

                if !character.is_whitespace() {
                    tokens.push(self.lex_token(&character.to_string())?);
                }

                token.drain(..);
            } else {
                token.push(character);
            }
        }

        Ok(tokens)
    }

    pub fn rewind(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.reader.rewind()?;

        Ok(())
    }

    pub fn next_line(&mut self) -> Result<Option<Vec<Token>>, Box<dyn std::error::Error>> {
        let mut line = String::new();

        if self.reader.read_line(&mut line)? != 0 {
            let tokens = self.lex_line(&line)?;

            if !tokens.is_empty() {
                Ok(Some(tokens))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(vec![Token::Eof]))
        }
    }
}

