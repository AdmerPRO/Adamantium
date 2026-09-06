use crate::syntax::{Call, Expr, Function, Operator, Program, Statement};

struct Generator {
    text: String,
    data: Vec<Vec<u8>>,
    next_slot: usize,
    max_slot: usize,
    label: usize,
}
fn memory(slot: usize) -> String {
    format!("[rbp - {}]", (slot + 1) * 8)
}
impl Generator {
    fn emit(&mut self, text: impl AsRef<str>) {
        self.text.push_str(text.as_ref());
        self.text.push('\n');
    }
    fn save(&mut self) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.max_slot = self.max_slot.max(self.next_slot);
        self.emit(format!("    mov {}, rax", memory(slot)));
        slot
    }
    fn call(&mut self, call: &Call) {
        let mark = self.next_slot;
        let mut slots = Vec::new();
        // Evaluate arguments left to right into frame-relative temporaries.
        for argument in &call.arguments {
            self.expression(argument);
            slots.push(self.save());
        }
        let size = (slots.len() * 8).div_ceil(16) * 16;
        // Large argument lists must also touch each Windows stack guard page.
        for offset in (0..size).step_by(4096) {
            let step = (size - offset).min(4096);
            self.emit(format!("    sub rsp, {step}\n    test byte [rsp], 0"));
        }
        for (i, slot) in slots.iter().enumerate() {
            self.emit(format!(
                "    mov rax, {}\n    mov [rsp + {}], rax",
                memory(*slot),
                i * 8
            ));
        }
        self.emit(format!("    call ad_fun_{}", call.name));
        if size != 0 {
            self.emit(format!("    add rsp, {size}"));
        }
        self.next_slot = mark;
    }
    fn expression(&mut self, expr: &Expr) {
        match expr {
            Expr::Integer(value) => self.emit(format!("    mov rax, {value}")),
            Expr::Variable(slot) => self.emit(format!("    mov rax, {}", memory(*slot))),
            Expr::Negate(expr) => {
                self.expression(expr);
                self.emit("    neg rax\n    jo ad_runtime_error");
            }
            Expr::Call(call) => self.call(call),
            Expr::Binary(operator, left, right) => {
                let mark = self.next_slot;
                self.expression(left);
                let slot = self.save();
                self.expression(right);
                self.emit(format!("    mov r10, rax\n    mov rax, {}", memory(slot)));
                match operator {
                    Operator::Add => self.emit("    add rax, r10\n    jo ad_runtime_error"),
                    Operator::Subtract => self.emit("    sub rax, r10\n    jo ad_runtime_error"),
                    Operator::Multiply => self.emit("    imul rax, r10\n    jo ad_runtime_error"),
                    Operator::Divide => {
                        self.label += 1;
                        self.emit(format!("    test r10, r10\n    jz ad_runtime_error\n    mov r11, -9223372036854775808\n    cmp rax, r11\n    jne .divide_safe_{}\n    cmp r10, -1\n    je ad_runtime_error\n.divide_safe_{}:\n    cqo\n    idiv r10", self.label, self.label));
                    }
                }
                self.next_slot = mark;
            }
        }
    }
    fn print_string(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let index = self.data.len();
        self.data.push(bytes.to_vec());
        self.emit(format!(
            "    lea rcx, [rel ad_string_{index}]\n    mov edx, {}\n    call ad_write",
            bytes.len()
        ));
    }
    fn function(&mut self, function: &Function) {
        self.next_slot = function.variables;
        self.max_slot = self.next_slot;
        let start = self.text.len();
        for slot in 0..function.parameters {
            self.emit(format!(
                "    mov rax, [rbp + {}]\n    mov {}, rax",
                16 + slot * 8,
                memory(slot)
            ));
        }
        for statement in &function.statements {
            match statement {
                Statement::Assign(slot, expr) => {
                    self.expression(expr);
                    self.emit(format!("    mov {}, rax", memory(*slot)));
                }
                Statement::Clamp(slot, low, high) => {
                    let mark = self.next_slot;
                    self.expression(low);
                    let low_slot = self.save();
                    self.expression(high);
                    self.emit(format!("    mov r10, {}\n    cmp r10, rax\n    jg ad_runtime_error\n    mov r11, {}\n    cmp r11, r10\n    cmovl r11, r10\n    cmp r11, rax\n    cmovg r11, rax\n    mov {}, r11", memory(low_slot), memory(*slot), memory(*slot)));
                    self.next_slot = mark;
                }
                Statement::PrintString(bytes) => self.print_string(bytes),
                Statement::PrintInteger(expr, newline) => {
                    self.expression(expr);
                    self.emit("    mov rcx, rax\n    call ad_print_integer");
                    if *newline {
                        self.print_string(b"\r\n");
                    }
                }
                Statement::Call(call) => self.call(call),
                Statement::Return => self.emit("    jmp .return"),
            }
        }
        self.emit(".return:");
        if let Some(slot) = function.result {
            self.emit(format!("    mov rax, {}", memory(slot)));
        } else {
            self.emit("    xor eax, eax");
        }
        self.emit("    mov rsp, rbp\n    pop rbp\n    ret");
        // Shadow space stays below locals and temporaries. Probe Windows guard pages.
        let frame = (self.max_slot * 8 + 32).div_ceil(16) * 16;
        let prologue = format!(
            "ad_fun_{}:\n    push rbp\n    mov rbp, rsp\n    mov r11, {frame}\n.probe:\n    cmp r11, 4096\n    jb .probe_tail\n    sub rsp, 4096\n    test byte [rsp], 0\n    sub r11, 4096\n    jmp .probe\n.probe_tail:\n    sub rsp, r11\n    test byte [rsp], 0\n",
            function.name
        );
        self.text.insert_str(start, &prologue);
    }
}
pub fn assembly(program: &Program) -> String {
    let mut generator = Generator {
        text: String::from(include_str!("runtime.asm")),
        data: Vec::new(),
        next_slot: 0,
        max_slot: 0,
        label: 0,
    };
    for function in &program.functions {
        generator.function(function);
    }
    generator.emit("section .rdata");
    for (i, bytes) in generator.data.iter().enumerate() {
        generator.text.push_str(&format!("ad_string_{i}:\n"));
        for chunk in bytes.chunks(32) {
            generator.text.push_str(&format!(
                "    db {}\n",
                chunk
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    generator.text
}
