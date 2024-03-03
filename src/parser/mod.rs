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
            Err("expected `,` between expressions".into())
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Register(Register),
    Integer(i32),
    Const(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConstExpr {
    Constant {
        ident: String,
        value: Value,
    },
    Macro {
        ident: String,
        args: Vec<String>,
        body: Vec<Inst>,
    },
    Call {
        ident: String,
        args: Vec<Value>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Inst {
    ConstExpr(ConstExpr),

    Label { ident: String },

    Jmp { label: String },
    Je { label: String },
    Jg { label: String },
    Jb { label: String },

    Push {
        value: Value,
    },
    Pop {
        dest: Register,
    },
    Mov {
        lhs: Value,
        rhs: Value,
    },
    Add {
        lhs: Value,
        rhs: Value,
    },
    Sub {
        lhs: Value,
        rhs: Value,
    },
    Mul {
        dest: Register,
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
                _ => Err(format!("no such instruction `{}`", ident).into()),
            }
        } else {
            Err(format!("no such instruction `{}`", ident).into())
        }
    }

    fn parse_expr(&mut self, expr: &[Token]) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(prefix) = expr.first() {
            match prefix {
                Token::Register(reg) => return Ok(Value::Register(*reg)),
                Token::Int(integer) => return Ok(Value::Integer(*integer)),
                Token::Ident(ident) => return Ok(Value::Const(ident.clone())),
                _ => return Err(format!("unexpected token `{:?}`", prefix).into()),
            }
        }

        Err("empty expression".into())
    }

    fn parse_reg(&mut self, expr: &[Token]) -> Result<Register, Box<dyn std::error::Error>> {
        if let Some(prefix) = expr.first() {
            match prefix {
                Token::Register(reg) => return Ok(*reg),
                _ => return Err("expected register".into()),
            }
        }

        Err("empty expression".into())
    }

    fn parse_jcc(&mut self, tokens: &[Token]) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(suffix) = tokens.first() {
            if let Token::Ident(label) = suffix {
                return Ok(label.clone());
            }
        }

        Err("expected label in jcc instruction".into())
    }

    fn parse_const_expr(&mut self, ident: String, tokens: &[Token]) -> Result<ConstExpr, Box<dyn std::error::Error>> {
        if tokens.len() < 3 {
            Err("empty expression".into())
        } else if tokens[1] != Token::Keyword(Keyword::Equ) {
            Err("expected `equ` in constexpr".into())
        } else if let Ok(value) = self.parse_expr(&[tokens[2].clone()]) {
            Ok(ConstExpr::Constant {
                ident: ident.clone(),
                value,
            })
        } else {
            Err("invalid expression".into())
        }
    }

    fn parse_ident(&mut self, token: &Token) -> Result<String, Box<dyn std::error::Error>> {
        if let Token::Ident(ident) = token {
            Ok(ident.clone())
        } else {
            Err("expected identifier".into())
        }
    }

    fn parse_args(&mut self, tokens: &[Token]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut args: Vec<String> = Vec::new();

        for token in tokens {
            if let Token::Ident(ident) = token {
                args.push(ident.clone());
            } else if *token != Token::Symbol(Symbol::Comma) {
                return Err(format!("unexpected token `{:?}`", token).into());
            }
        }

        Ok(args)
    }

    fn parse_macro(&mut self, tokens: &[Token]) -> Result<ConstExpr, Box<dyn std::error::Error>> {
        if tokens.len() < 2 {
            Err("invalid macro expression\nusage:\nmacro <IDENT> [ARGS]\n{\n    <body>\n}".into())
        } else {
            let ident = self.parse_ident(&tokens[0])?;
            let args = self.parse_args(&tokens[1..])?;

            let mut line = self.lexer.next_line()?;
            let mut body: Vec<Inst> = Vec::new();

            loop {
                if let Some(line) = &line {
                    if let Some(prefix) = line.get(0) {
                        if *prefix == Token::Symbol(Symbol::CloseBrace) || *prefix == Token::Eof {
                            return Ok(ConstExpr::Macro {
                                ident,
                                args,
                                body,
                            });
                        }

                        if *prefix != Token::Symbol(Symbol::OpenBrace) {
                            if let Some(inst) = self.parse_line(line.clone())? {
                                body.push(inst);
                            }
                        }
                    }
                }

                line = self.lexer.next_line()?;
            }
        }
    }

    fn parse_call(&mut self, ident: String, tokens: &[Token]) -> Result<ConstExpr, Box<dyn std::error::Error>> {
        let mut args: Vec<Value> = Vec::new();

        for token in &tokens[1..] {
            if *token != Token::Symbol(Symbol::Comma) {
                args.push(self.parse_expr(&[token.clone()])?);
            }
        }

        Ok(ConstExpr::Call {
            ident,
            args,
        })
    }

    fn parse_line(&mut self, mut tokens: Vec<Token>) -> Result<Option<Inst>, Box<dyn std::error::Error>> {
        if let Some(prefix) = tokens.clone().first() {
            return match prefix {
                Token::Ident(ident) => {
                    if let Ok(constexpr) = self.parse_const_expr(ident.clone(), &tokens) {
                        Ok(Some(Inst::ConstExpr(constexpr)))
                    } else if let Ok(constexpr) = self.parse_call(ident.clone(), &tokens) {
                        Ok(Some(Inst::ConstExpr(constexpr)))
                    } else {
                        Ok(Some(self.parse_label(ident, &tokens)?))
                    }
                },
                Token::Keyword(keyword) => {
                    tokens.remove(0);

                    match keyword {
                        Keyword::Push => Ok(Some(Inst::Push {
                            value: self.parse_expr(&tokens)?,
                        })),
                        Keyword::Pop => Ok(Some(Inst::Pop {
                            dest: self.parse_reg(&tokens)?,
                        })),
                        Keyword::Mov => Ok(Some(Inst::Mov {
                            lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                            rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?
                        })),
                        Keyword::Add => Ok(Some(Inst::Add {
                            lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                            rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?,
                        })),
                        Keyword::Sub => Ok(Some(Inst::Sub {
                            lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                            rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?,
                        })),
                        Keyword::Mul => Ok(Some(Inst::Mul {
                            dest: self.parse_reg(&tokens)?,
                        })),
                        Keyword::Cmp => Ok(Some(Inst::Cmp {
                            lhs: self.parse_expr(&SplitTokens::new(&tokens)?.lhs)?,
                            rhs: self.parse_expr(&SplitTokens::new(&tokens)?.rhs)?,
                        })),
                        Keyword::Jmp => Ok(Some(Inst::Jmp { label: self.parse_jcc(&tokens)? })),
                        Keyword::Je => Ok(Some(Inst::Je { label: self.parse_jcc(&tokens)? })),
                        Keyword::Jg => Ok(Some(Inst::Jg { label: self.parse_jcc(&tokens)? })),
                        Keyword::Jb => Ok(Some(Inst::Jb { label: self.parse_jcc(&tokens)? })),
                        Keyword::Syscall => Ok(Some(Inst::Syscall)),

                        Keyword::Macro => Ok(Some(Inst::ConstExpr(self.parse_macro(&tokens)?))),
                        _ => Err(format!("unexpected token `{:?}`", keyword).into()),
                    }
                },
                Token::Eof => Ok(Some(Inst::Eof)),
                _ => Err(format!("unexpected token `{:?}`", prefix).into()),
            };
        }

        Ok(None)
    }

    pub fn next_inst(&mut self) -> Result<Option<Inst>, Box<dyn std::error::Error>> {
        if let Some(tokens) = self.lexer.next_line()? {
            return self.parse_line(tokens);
        }

        Ok(None)
    }
}

