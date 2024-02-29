use crate::parser::lexer::Register;
use crate::parser::Parser;
use crate::parser::Value;
use crate::parser::Inst;
use crate::preprocessor::Preprocessor;

use faerie::{ArtifactBuilder, Artifact};
use target_lexicon::triple;

use std::process::Command;
use std::str::FromStr;
use std::fs::File;

pub struct Codegen {
    obj: Artifact,
    parser: Parser,
    preprocessor: Preprocessor,
    buf: Vec<u8>,
    label: String,
    pub line: usize,
}

impl Codegen {
    pub fn new(file: &str) -> Result<Codegen, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(file)?;
        let mut preprocessor = Preprocessor::new();

        preprocessor.preprocess(&mut parser)?;
        parser.lexer.rewind()?;

        println!("offsets: {:#x?}", preprocessor.offsets);

        Ok(Codegen {
            obj: ArtifactBuilder::new(triple!("x86_64-unknown-unknown-unknown-elf"))
                .name(file.to_string())
                .finish(),
            parser,
            buf: Vec::new(),
            preprocessor,
            label: String::new(),
            line: 1,
        })
    }

    fn to_bytes(integer: i32) -> Vec<u8> {
        if cfg!(target_endian = "big") {
            integer.to_be_bytes().to_vec()
        } else {
            integer.to_le_bytes().to_vec()
        }
    }

    fn rm(reg: Register) -> u8 {
        match reg {
            Register::Eax => 0,
            Register::Ebx => 3,
            Register::Ecx => 1,
            Register::Edx => 2,
            Register::Esi => 6,
            Register::Edi => 7,
            Register::Esp => 4,
            Register::Ebp => 5,
        }
    }

    // https://en.wikipedia.org/wiki/ModR/M
    // page 44 @ intel programmers manual
    fn format_modrm(mod_: u8, reg: u8, rm: u8) -> u8 {
        (mod_ << 6) | (reg << 3) | rm
    }

    fn define_label(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.label.is_empty() {
            self.obj.define(self.label.clone(), self.buf.clone())?;
            self.buf.drain(..);
        }

        Ok(())
    }

    fn encode_jcc(&mut self, opcode: &[u8], label: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(addr) = self.preprocessor.offsets.get(&label) {
            self.preprocessor.offset += opcode.len() + 4;
            self.buf.extend(&[opcode.to_vec(), Self::to_bytes((*addr as i32) - self.preprocessor.offset as i32)].concat());

            Ok(())
        } else {
            Err(format!("> error: no such label `{}`", label).into())
        }
    }

    fn build(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.obj.declarations(self.preprocessor.labels.iter().cloned())?;

        let mut inst = self.parser.next_inst()?;

        loop {
            self.line += 1;

            if let Some(inst) = inst {
                match inst {
                    Inst::Label { ident } => {
                        self.define_label()?;

                        self.label = ident;
                    },
                    Inst::Mov { lhs, rhs } => {
                        // file:///home/proxin/Downloads/325383-sdm-vol-2abcd.pdf#page=769
                        if let Value::Register(rd) = lhs {
                            if let Value::Integer(id) = rhs {
                                // B8+ rd id
                                self.buf.extend(&[vec![0xb8 + Self::rm(rd)], Self::to_bytes(id)].concat());
                                self.preprocessor.offset += 5;
                            } else if let Value::Register(id) = rhs {
                                // 89 /r
                                self.buf.extend(&[0x89, Self::format_modrm(3, Self::rm(id), Self::rm(rd))]);
                                self.preprocessor.offset += 2;
                            }
                        } else {
                            return Err("> error: cant move into non-register".into());
                        }
                    },
                    Inst::Add { lhs, rhs } => {
                        if let Value::Register(rd) = lhs {
                            if let Value::Integer(id) = rhs {
                                if rd == Register::Eax {
                                    // 05 id
                                    self.buf.extend(&[vec![0x05], Self::to_bytes(id)].concat());
                                    self.preprocessor.offset += 5;
                                } else {
                                    // 81 /0 id
                                    self.buf.extend(&[vec![0x81, Self::format_modrm(3, 0, Self::rm(rd))], Self::to_bytes(id)].concat());
                                    self.preprocessor.offset += 6;
                                }
                            } else if let Value::Register(id) = rhs {
                                // 01 /r
                                self.buf.extend(&[0x01, Self::format_modrm(3, Self::rm(id), Self::rm(rd))]);
                                self.preprocessor.offset += 2;
                            }
                        } else {
                            return Err("> error: cant add into non-register".into());
                        }
                    },
                    Inst::Cmp { lhs, rhs } => {
                        if let Value::Register(rd) = lhs {
                            if let Value::Integer(id) = rhs {
                                // 81 /7 id
                                self.buf.extend(&[vec![0x81, Self::format_modrm(3, 7, Self::rm(rd))], Self::to_bytes(id)].concat());
                                self.preprocessor.offset += 6;
                            } else if let Value::Register(id) = rhs {
                                // 39 /r
                                self.buf.extend(&[0x39, Self::format_modrm(3, Self::rm(id), Self::rm(rd))]);
                                self.preprocessor.offset += 2;
                            }
                        } else {
                            return Err("> error: cant cmp non register".into());
                        }
                    },
                    Inst::Jmp { label } => self.encode_jcc(&[0xe9], label)?,
                    Inst::Je { label } => self.encode_jcc(&[0x0f, 0x84], label)?,
                    Inst::Syscall => {
                        self.buf.extend(&[0x0f, 0x05]);
                        self.preprocessor.offset += 2;
                    },
                    Inst::Eof => {
                        self.define_label()?;

                        break;
                    },
                }
            }

            inst = self.parser.next_inst()?;
        }

        Ok(())
    }

    pub fn emit(&mut self, file: &str) -> Result<(), Box<dyn std::error::Error>> {
        let fd = File::create(file)?;

        self.build()?;
        self.obj.write(fd)?;

        Command::new("ld")
            .args(["-o", "program", "program.o"])
            .spawn()?;

        Ok(())
    }
}

