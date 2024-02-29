pub mod lexer;

use lexer::Register;
use lexer::Keyword;
use lexer::Symbol;
use lexer::Token;
use lexer::Lexer;

pub struct SplitTokens {
    lhs: Vec<Token>,
    rhs: Vec<Token>,
}

impl SplitTokens {
    pub fn new(tokens: &[Token]) -> Result<SplitTokens, Box<dyn std::error::Error>> {
        if let Ok(comma) = tokens.binary_search(&Token::Symbol(Symbol::Comma)) {
            Ok(SplitTokens {
                lhs: tokens[..comma].to_vec(),
                rhs: tokens[comma + 1..].to_vec(),
            })
        } else {
            Err("> error: expected `,` between expressions".into())
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Register(Register),
    Integer(i32),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Inst {
    Label { ident: String },

    Jmp { label: String },
    Je { label: String },

    Mov {
        lhs: Value,
        rhs: Value,
    },
    Add {
        lhs: Value,
        rhs: Value,
    },
    Cmp {
        lhs: Value,
        rhs: Value,
    },
    Syscall,

    Eof,
}

pub struct Parser {
    pub lexer: Lexer,
}

impl Parser {
    pub fn new(file: &str) -> Result<Parser, Box<dyn std::error::Error>> {
        Ok(Parser {
            lexer: Lexer::new(file)?,
        })
    }

    fn parse_label(&mut self, ident: &String, tokens: &[Token]) -> Result<Inst, Box<dyn std::error::Error>> {
        if let Some(token) = tokens.get(1) {
            match token {
                Token::Symbol(Symbol::Colon) => Ok(Inst::Label { ident: ident.to_string() }),
                _ => Err(format!("> error: no such instruction `{}`", ident).into()),
            }
        } else {
            Err(format!("> error: no such instruction `{}`", ident).into())
        }
    }

    fn parse_expr(&mut self, expr: &[Token]) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(prefix) = expr.first() {
            match prefix {
                Token::Register(reg) => return Ok(Value::Register(*reg)),
                Token::Int(integer) => return Ok(Value::Integer(*integer)),
                _ => return Err(format!("> error: unexpected token `{:?}`", prefix).into()),
            }
        }

        Err("> error: empty expression".into())
    }

    fn parse_jcc(&mut self, tokens: &[Token]) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(suffix) = tokens.first() {
            if let Token::Ident(label) = suffix {
                return Ok(label.clone());
            }
        }

        Err("> error: expected label in jcc instruction".into())
    }

    pub fn next_inst(&mut self) -> Result<Option<Inst>, Box<dyn std::error::Error>> {
        if let Some(mut tokens) = self.lexer.next_line()? {
            if let Some(prefix) = tokens.clone().first() {
                return match prefix {
                    Token::Ident(ident) => Ok(Some(self.parse_label(ident, &tokens)?)),
                    Token::Keyword(keyword) => {
                        tokens.remove(0);

                        match keyword {
                            Keyword::Mov => Ok(Some(Inst::Mov {
                                lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                                rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?
                            })),
                            Keyword::Add => Ok(Some(Inst::Add {
                                lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                                rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?,
                            })),
                            Keyword::Cmp => Ok(Some(Inst::Cmp {
                                lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                                rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?,
                            })),
                            Keyword::Jmp => Ok(Some(Inst::Jmp { label: self.parse_jcc(&tokens)? })),
                            Keyword::Je => Ok(Some(Inst::Je { label: self.parse_jcc(&tokens)? })),
                            Keyword::Syscall => Ok(Some(Inst::Syscall)),
                        }
                    },
                    Token::Eof => Ok(Some(Inst::Eof)),
                    _ => Err(format!("> error: unexpected token `{:?}`", prefix).into()),
                };
            }
        }

        Ok(None)
    }
}

