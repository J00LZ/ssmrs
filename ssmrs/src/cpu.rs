use std::{
    cmp::{max, min},
    collections::HashMap,
    fmt::Formatter,
};

use crate::{
    instruction::Instr,
    register::{Reg, RegisterFile},
    Code,
};

// #[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    memory: [i32; 2048],
    registers: RegisterFile,
    verbosity: u8,
    write: Box<dyn Fn(String)>,
}

impl std::fmt::Debug for Cpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cpu")
            .field("memory", &self.memory)
            .field("registers", &self.registers)
            .field("verbosity", &self.verbosity)
            .finish()
    }
}

impl Cpu {
    pub fn new(verbosity: u8, write: Box<dyn Fn(String)>) -> Cpu {
        Cpu {
            memory: [0; 2048],
            registers: RegisterFile::new(),
            verbosity,
            write,
        }
    }

    pub fn load_code(&mut self, mut code: Code) {
        fix_jumps(&mut code);
        remove_annote(&mut code);
        let code = convert_code(&code);
        for (i, v) in code.iter().enumerate() {
            self.memory[i] = *v;
        }
        self.set_reg(Reg::PC, 0);
        self.set_reg(Reg::SP, code.len() as i32);
    }

    fn get_reg(&self, reg: Reg) -> i32 {
        self.registers[reg]
    }

    fn set_reg(&mut self, reg: Reg, val: i32) {
        self.registers[reg] = val;
    }

    fn adjust_reg(&mut self, reg: Reg, val: i32) {
        self.registers[reg] += val;
    }

    fn get_mem(&self, addr: i32) -> i32 {
        self.memory[addr as usize]
    }

    fn set_mem(&mut self, addr: i32, val: i32) {
        self.memory[addr as usize] = val;
    }

    fn set_mem_reg(&mut self, reg: Reg, val: i32) {
        self.set_mem(self.get_reg(reg), val);
    }

    fn get_mem_reg(&self, reg: Reg) -> i32 {
        self.get_mem(self.get_reg(reg))
    }

    pub fn read_memory(&self) -> &[i32] {
        &self.memory
    }

    pub fn read_registers(&self) -> &RegisterFile {
        &self.registers
    }

    pub fn step(&mut self) -> bool {
        let current_pc = self.get_reg(Reg::PC);
        let instr = Instr::from(&self.memory[current_pc as usize..current_pc as usize + 3]);
        if self.verbosity > 1 {
            (self.write)(format!("Registers: {:?}", self.registers));
            (self.write)(format!(
                "Memory: {:?}",
                &self.memory[0..(self.get_reg(Reg::SP) as usize + 1)]
            ));
        }
        if self.verbosity > 0 {
            (self.write)(format!("Executing {:?}", instr));
        }
        self.set_reg(Reg::PC, current_pc + instr.instr_size() as i32);
        self.exec(instr)
    }

    fn push_stack(&mut self, value: i32) {
        self.adjust_reg(Reg::SP, 1);
        self.set_mem_reg(Reg::SP, value);
    }

    fn pop_stack(&mut self) -> i32 {
        let s = self.get_mem_reg(Reg::SP);
        self.adjust_reg(Reg::SP, -1);
        s
    }

    fn exec(&mut self, i: Instr) -> bool {
        match i {
            Instr::STR(reg) => {
                let v = self.pop_stack();
                self.set_reg(reg, v);
            }
            Instr::STL(rel) => {
                let addr = self.get_reg(Reg::MP) + rel;
                let val = self.pop_stack();
                self.set_mem(addr, val);
            }
            Instr::STS(rel) => {
                let addr = self.get_reg(Reg::SP) + rel;
                let val = self.pop_stack();
                self.set_mem(addr, val);
            }
            Instr::STA(rel) => {
                let addr = self.pop_stack();
                let value = self.pop_stack();
                self.set_mem(addr + rel, value);
            }
            Instr::LDR(reg) => {
                let v = self.get_reg(reg);
                self.push_stack(v);
            }
            Instr::LDL(rel) => {
                let addr = self.get_reg(Reg::MP) + rel;
                let v = self.get_mem(addr);
                self.push_stack(v);
            }
            Instr::LDS(rel) => {
                let addr = self.get_reg(Reg::SP) + rel;
                let v = self.get_mem(addr);
                self.push_stack(v);
            }
            Instr::LDA(addr) => {
                let p = self.pop_stack();
                let v = self.get_mem(p + addr);
                self.push_stack(v);
            }
            Instr::LDC(n) => {
                self.push_stack(n);
            }
            Instr::LDLA(rel) => {
                let addr = self.get_reg(Reg::MP) + rel;
                self.push_stack(addr);
            }
            Instr::LDSA(rel) => {
                let addr = self.get_reg(Reg::SP) + rel;
                self.push_stack(addr);
            }
            Instr::LDAA(rel) => {
                let addr = self.pop_stack() + rel;
                self.push_stack(addr);
            }
            Instr::BRA(rel) => {
                let addr = self.get_reg(Reg::PC) + rel;
                self.set_reg(Reg::PC, addr);
            }
            Instr::BRF(rel) => {
                let addr = self.get_reg(Reg::PC) + rel;
                let cond = self.pop_stack();
                if cond == 0 {
                    self.set_reg(Reg::PC, addr);
                }
            }
            Instr::BRT(rel) => {
                let addr = self.get_reg(Reg::PC) + rel;
                let cond = self.pop_stack();
                if cond != 0 {
                    self.set_reg(Reg::PC, addr);
                }
            }
            Instr::BSR(rel) => {
                let addr = self.get_reg(Reg::PC) + rel;
                self.push_stack(self.get_reg(Reg::PC));
                self.set_reg(Reg::PC, addr);
            }
            Instr::ADD => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack(a + b);
            }
            Instr::SUB => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack(a - b);
            }
            Instr::MUL => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack(a * b);
            }
            Instr::DIV => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack(a / b);
            }
            Instr::MOD => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack(a % b);
            }
            Instr::EQ => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack((a == b).get_ssm_value());
            }
            Instr::NE => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack((a != b).get_ssm_value());
            }
            Instr::LT => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack((a < b).get_ssm_value());
            }
            Instr::LE => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack((a <= b).get_ssm_value());
            }
            Instr::GT => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack((a > b).get_ssm_value());
            }
            Instr::GE => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.push_stack((a >= b).get_ssm_value());
            }
            Instr::NEG => {
                let a = self.pop_stack();
                self.push_stack(-a);
            }
            Instr::NOT => {
                let a = self.pop_stack();
                self.push_stack((a == 0).get_ssm_value());
            }
            Instr::RET => {
                let addr = self.pop_stack();
                self.set_reg(Reg::PC, addr);
            }
            Instr::UNLINK => {
                let old_mp = self.get_reg(Reg::MP);
                self.set_reg(Reg::SP, old_mp);
                let new_mp = self.pop_stack();
                self.set_reg(Reg::MP, new_mp);
            }
            Instr::LINK(locals) => {
                let mp = self.get_reg(Reg::MP);
                self.push_stack(mp);
                let sp = self.get_reg(Reg::SP);
                self.set_reg(Reg::MP, sp);
                self.adjust_reg(Reg::SP, locals);
            }
            Instr::AJS(rel) => {
                let sp = self.get_reg(Reg::SP);
                self.set_reg(Reg::SP, sp + rel);
            }
            Instr::SWP => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack(a);
                self.push_stack(b);
            }
            Instr::SWPR(reg) => {
                let a = self.pop_stack();
                let b = self.get_reg(reg);
                self.push_stack(a);
                self.set_reg(reg, b);
            }
            Instr::SWPRR(reg1, reg2) => {
                let a = self.get_reg(reg1);
                let b = self.get_reg(reg2);
                self.set_reg(reg1, b);
                self.set_reg(reg2, a);
            }
            Instr::LDRR(dest, src) => {
                let v = self.get_reg(src);
                self.set_reg(dest, v);
            }
            Instr::JSR => {
                let addr = self.pop_stack();
                self.push_stack(self.get_reg(Reg::PC));
                self.set_reg(Reg::PC, addr);
            }
            Instr::TRAP(op) => match op {
                0 => {
                    let v = self.pop_stack();
                    (self.write)(format!("{}", v));
                }
                1 => {
                    let v = self.pop_stack();
                    if let Some(chr) = char::from_u32(v as u32) {
                        (self.write)(format!("{}", chr));
                    }
                }
                _ => panic!("Unknown trap: {}", op),
            },
            Instr::NOP => {}
            Instr::HALT => return false,
            Instr::AND => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack(a & b);
            }
            Instr::OR => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack(a | b);
            }
            Instr::XOR => {
                let a = self.pop_stack();
                let b = self.pop_stack();
                self.push_stack(a ^ b);
            }
            _ => panic!("Invalid instruction!"),
        }
        true
    }
}

trait GetSSMValue {
    fn get_ssm_value(&self) -> i32;
}

impl GetSSMValue for bool {
    fn get_ssm_value(&self) -> i32 {
        if *self {
            -1
        } else {
            0
        }
    }
}

fn fix_jumps(code: &mut Code) {
    let mut labels = HashMap::new();
    for (i, instr) in code.iter().enumerate() {
        if let Instr::LABEL(n) = instr {
            labels.insert(n.clone(), i);
        }
    }
    let sizes = code.iter().map(Instr::instr_size).collect::<Vec<_>>();
    for (current_idx, instr) in code.iter_mut().enumerate() {
        let current_idx = current_idx + 1;
        match instr {
            Instr::Bra(n) => {
                let target = *labels.get(n).unwrap();
                let start = min(target, current_idx);
                let end = max(target, current_idx);
                let size = sizes[start..=end].iter().sum::<usize>() as i32;
                if target < current_idx {
                    *instr = Instr::BRA(2-size);
                } else {
                    *instr = Instr::BRA(size);
                }
            }
            Instr::Brt(n) => {
                let target = *labels.get(n).unwrap();
                let start = min(target, current_idx);
                let end = max(target, current_idx);
                let size = sizes[start..=end].iter().sum::<usize>() as i32;
                if target < current_idx {
                    *instr = Instr::BRT(2-size);
                } else {
                    *instr = Instr::BRT(size);
                }
            }
            Instr::Brf(n) => {
                let target = *labels.get(n).unwrap();
                let start = min(target, current_idx);
                let end = max(target, current_idx);
                let size = sizes[start..=end].iter().sum::<usize>() as i32;
                if target < current_idx {
                    *instr = Instr::BRF(2-size);
                } else {
                    *instr = Instr::BRF(size);
                }
            }
            Instr::Bsr(n) => {
                let target = *labels.get(n).unwrap();
                let start = min(target, current_idx);
                let end = max(target, current_idx);
                let size = sizes[start..=end].iter().sum::<usize>() as i32;
                if target < current_idx {
                    *instr = Instr::BSR(2-size);
                } else {
                    *instr = Instr::BSR(size);
                }
            }
            _ => (),
        }
    }
    code.retain(|instr| !matches!(instr, Instr::LABEL(_)));
}

fn remove_annote(code: &mut Code) {
    code.retain(|instr| !matches!(instr, Instr::ANNOTE(_, _, _, _, _)));
}

fn convert_code(code: &Code) -> Vec<i32> {
    code.iter().flat_map(Instr::convert).collect()
}
