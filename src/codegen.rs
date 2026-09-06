use crate::{
    syntax::Operator,
    typed::{Expression, Function, Instruction, Kind, Program},
    types::Type,
};

struct Generator {
    text: String,
    data: Vec<Vec<u8>>,
    next_slot: usize,
    max_slot: usize,
}
fn memory(slot: usize, offset: usize) -> String {
    format!("[rbp - {}]", (slot + 1) * 16 - offset)
}
impl Generator {
    fn emit(&mut self, text: impl AsRef<str>) {
        self.text.push_str(text.as_ref());
        self.text.push('\n');
    }
    fn reserve(&mut self, count: usize) -> usize {
        self.next_slot += count;
        self.max_slot = self.max_slot.max(self.next_slot);
        self.next_slot - 1
    }
    fn store(&mut self, slot: usize) {
        self.emit(format!(
            "    mov {}, rax\n    mov {}, rdx",
            memory(slot, 0),
            memory(slot, 8)
        ));
    }
    fn load(&mut self, slot: usize) {
        self.emit(format!(
            "    mov rax, {}\n    mov rdx, {}",
            memory(slot, 0),
            memory(slot, 8)
        ));
    }
    fn save(&mut self) -> usize {
        let slot = self.reserve(1);
        self.store(slot);
        slot
    }
    fn call(&mut self, name: &str, arguments: &[Expression]) {
        let mark = self.next_slot;
        let mut slots = Vec::new();
        for argument in arguments {
            self.expression(argument);
            slots.push(self.save());
        }
        let size = slots.len() * 16;
        for offset in (0..size).step_by(4096) {
            let step = (size - offset).min(4096);
            self.emit(format!("    sub rsp, {step}\n    test byte [rsp], 0"));
        }
        for (i, slot) in slots.iter().enumerate() {
            self.load(*slot);
            self.emit(format!(
                "    mov [rsp + {}], rax\n    mov [rsp + {}], rdx",
                i * 16,
                i * 16 + 8
            ));
        }
        self.emit(format!("    call ad_fun_{name}"));
        if size != 0 {
            self.emit(format!("    add rsp, {size}"));
        }
        self.next_slot = mark;
    }
    fn evaluate(&mut self, operation: u32, ty: Type, from: Type, operands: &[usize]) {
        let mark = self.next_slot;
        let request = self.reserve(5);
        // Request is 80 bytes: three operands, output, operation/type/from/padding.
        for offset in (0..80).step_by(8) {
            self.emit(format!("    mov qword {}, 0", memory(request, offset)));
        }
        for (i, slot) in operands.iter().enumerate() {
            self.load(*slot);
            self.emit(format!(
                "    mov {}, rax\n    mov {}, rdx",
                memory(request, i * 16),
                memory(request, i * 16 + 8)
            ));
        }
        self.emit(format!("    mov dword {}, {operation}\n    mov dword {}, {}\n    mov dword {}, {}\n    lea rcx, {}\n    call ad_evaluate\n    test eax, eax\n    jnz ad_exit_error\n    mov rax, {}\n    mov rdx, {}", memory(request,64), memory(request,68),ty.id(),memory(request,72),from.id(),memory(request,0),memory(request,48),memory(request,56)));
        self.next_slot = mark;
    }
    fn expression(&mut self, expr: &Expression) {
        match &expr.kind {
            Kind::Constant(value) => self.emit(format!(
                "    mov rax, {}\n    mov rdx, {}",
                value.lo, value.hi
            )),
            Kind::String(bytes) => {
                let index = self.data.len();
                self.data.push(bytes.clone());
                self.emit(format!(
                    "    lea rax, [rel ad_string_{index}]\n    mov rdx, {}",
                    bytes.len()
                ));
            }
            Kind::Variable(slot) => self.load(*slot),
            Kind::Call(name, arguments) => self.call(name, arguments),
            Kind::Negate(value) | Kind::Convert(value) => {
                let mark = self.next_slot;
                self.expression(value);
                let slot = self.save();
                let op = if matches!(expr.kind, Kind::Negate(_)) {
                    4
                } else {
                    5
                };
                self.evaluate(op, expr.ty, value.ty, &[slot]);
                self.next_slot = mark;
            }
            Kind::Binary(op, a, b) => {
                let mark = self.next_slot;
                self.expression(a);
                let left = self.save();
                self.expression(b);
                let right = self.save();
                let op = match op {
                    Operator::Add => 0,
                    Operator::Subtract => 1,
                    Operator::Multiply => 2,
                    Operator::Divide => 3,
                };
                self.evaluate(op, expr.ty, expr.ty, &[left, right]);
                self.next_slot = mark;
            }
        }
    }
    fn function(&mut self, function: &Function) {
        self.next_slot = function.types.len();
        self.max_slot = self.next_slot;
        let start = self.text.len();
        for slot in 0..function.parameters {
            self.emit(format!(
                "    mov rax, [rbp + {}]\n    mov rdx, [rbp + {}]",
                16 + slot * 16,
                24 + slot * 16
            ));
            self.store(slot);
        }
        for instruction in &function.instructions {
            let mark = self.next_slot;
            match instruction {
                Instruction::Assign(slot, value) => {
                    self.expression(value);
                    self.store(*slot);
                }
                Instruction::Clamp(slot, low, high) => {
                    self.expression(low);
                    let low = self.save();
                    self.expression(high);
                    let high = self.save();
                    self.evaluate(
                        6,
                        function.types[*slot],
                        function.types[*slot],
                        &[*slot, low, high],
                    );
                    self.store(*slot);
                }
                Instruction::Print(value, newline) => {
                    self.expression(value);
                    let slot = self.save();
                    self.emit(format!("    lea rcx, {}\n    mov edx, {}\n    mov r8d, {}\n    call ad_print\n    test eax, eax\n    jnz ad_exit_error",memory(slot,0),value.ty.id(),u8::from(*newline)));
                }
                Instruction::Call(expr) => self.expression(expr),
                Instruction::Return => self.emit("    jmp .return"),
            }
            self.next_slot = mark;
        }
        self.emit(".return:");
        if let Some(slot) = function.result {
            self.load(slot);
        } else {
            self.emit("    xor eax, eax\n    xor edx, edx");
        }
        self.emit("    mov rsp, rbp\n    pop rbp\n    ret");
        let frame = self.max_slot * 16 + 32;
        self.text.insert_str(start,&format!("ad_fun_{}:\n    push rbp\n    mov rbp, rsp\n    mov r11, {frame}\n.probe:\n    cmp r11, 4096\n    jb .tail\n    sub rsp, 4096\n    test byte [rsp], 0\n    sub r11, 4096\n    jmp .probe\n.tail:\n    sub rsp, r11\n    test byte [rsp], 0\n",function.name));
    }
}
pub fn assembly(program: &Program) -> String {
    let mut generator = Generator {
        text: include_str!("runtime.asm").into(),
        data: Vec::new(),
        next_slot: 0,
        max_slot: 0,
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
        generator.text.push_str("    db 0\n");
    }
    generator.text
}
