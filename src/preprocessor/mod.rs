use crate::parser::lexer::Register;
use crate::parser::Parser;
use crate::parser::Value;
use crate::parser::Inst;

use faerie::Decl;

use std::collections::HashMap;

pub struct Preprocessor {
    pub labels: Vec<(String, Decl)>,
    pub offsets: HashMap<String, usize>,
    pub offset: usize,
}

impl Preprocessor {
    pub fn new() -> Preprocessor {
        Preprocessor {
            labels: Vec::new(),
            offsets: HashMap::new(),
            offset: 0x401000,
        }
    }

    pub fn preprocess(&mut self, parser: &mut Parser) -> Result<(), Box<dyn std::error::Error>> {
        let mut inst = parser.next_inst();

        loop {
            if let Ok(inst) = inst {
                if let Some(inst) = inst {
                    match inst {
                        Inst::Label { ident } => {
                            self.offsets.insert(ident.clone(), self.offset);
                            self.labels.push((ident, Decl::function().global().with_align(Some(1)).into()));
                        },
                        Inst::Push { value } => {
                            if let Value::Integer(_) = value {
                                self.offset += 5;
                            } else if let Value::Register(_) = value {
                                self.offset += 2;
                            }
                        },
                        Inst::Pop { .. } => self.offset += 1,
                        Inst::Mov { rhs, .. } => {
                            if let Value::Integer(_) = rhs {
                                self.offset += 5;
                            } else if let Value::Register(_) = rhs {
                                self.offset += 2;
                            }
                        },
                        Inst::Add { rhs, lhs } | Inst::Sub { rhs, lhs } => {
                            if let Value::Register(rd) = lhs {
                                if let Value::Integer(_) = rhs {
                                    if rd == Register::Eax {
                                        self.offset += 5;
                                    } else {
                                        self.offset += 6;
                                    }
                                } else if let Value::Register(_) = rhs {
                                    self.offset += 2;
                                }
                            }
                        },
                        Inst::Mul { .. } => self.offset += 2,
                        Inst::Cmp { lhs, rhs } => {
                            if let Value::Register(_) = lhs {
                                if let Value::Integer(_) = rhs {
                                    self.offset += 6;
                                } else if let Value::Register(_) = rhs {
                                    self.offset += 2;
                                }
                            }
                        },
                        Inst::Jmp { .. } => self.offset += 5,
                        Inst::Je { .. } | Inst::Jg { .. } | Inst::Jb { .. } => self.offset += 6,
                        Inst::Syscall => self.offset += 2,

                        Inst::Eof => {
                            break
                        },
                    }
                }
            }

            inst = parser.next_inst();
        }

        self.offset = 0x401000;

        Ok(())
    }
}

