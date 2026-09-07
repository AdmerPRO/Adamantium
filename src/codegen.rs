use crate::{
    syntax::{Comparison, Operator},
    typed::{Expression, Function, Instruction, Kind, Program},
    types::Type,
};

struct Generator {
    text: String,
    data: Vec<Vec<u8>>,
    next_slot: usize,
    max_slot: usize,
    class_sizes: Vec<usize>,
    next_label: usize,
    loop_stack: Vec<(String, String)>,
}
fn memory(slot: usize, offset: usize) -> String {
    format!("[rbp - {}]", (slot + 1) * 16 - offset)
}
impl Generator {
    fn label(&mut self, prefix: &str) -> String {
        let label = format!("ad_{prefix}_{}", self.next_label);
        self.next_label += 1;
        label
    }
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
    fn clone_class(&mut self, ty: Type) {
        if let Type::Class(id) = ty {
            self.emit(format!(
                "    mov rcx, rax\n    mov edx, {}\n    call ad_object_clone\n    xor edx, edx",
                self.class_sizes[id as usize]
            ));
        }
    }
    fn call(&mut self, name: &str, arguments: &[Expression]) {
        let mark = self.next_slot;
        let mut slots = Vec::new();
        for argument in arguments {
            self.expression(argument);
            self.clone_class(argument.ty);
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
            Kind::Construct(id, fields, constructor) => {
                let mark = self.next_slot;
                let mut values = Vec::new();
                for field in fields {
                    self.expression(field);
                    self.clone_class(field.ty);
                    values.push(self.save());
                }
                self.emit(format!(
                    "    mov ecx, {}\n    call ad_object_new\n    xor edx, edx",
                    fields.len()
                ));
                let object = self.save();
                for (index, value) in values.iter().enumerate() {
                    self.load(object);
                    self.emit("    mov r11, rax");
                    self.load(*value);
                    self.emit(format!(
                        "    mov [r11 + {}], rax\n    mov [r11 + {}], rdx",
                        index * 16,
                        index * 16 + 8
                    ));
                }
                self.load(object);
                let receiver = self.save();
                self.call_saved(constructor, &[receiver]);
                self.load(object);
                self.next_slot = mark;
                let _ = id;
            }
            Kind::Field(object, index) => {
                self.expression(object);
                self.emit(format!(
                    "    mov r11, rax\n    mov rax, [r11 + {}]\n    mov rdx, [r11 + {}]",
                    index * 16,
                    index * 16 + 8
                ));
            }
            Kind::MethodCall(name, object, arguments) => {
                let mark = self.next_slot;
                self.expression(object);
                let mut slots = vec![self.save()];
                for argument in arguments {
                    self.expression(argument);
                    self.clone_class(argument.ty);
                    slots.push(self.save());
                }
                self.call_saved(name, &slots);
                self.next_slot = mark;
            }
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
            Kind::Compare(comparison, a, b) => {
                let mark = self.next_slot;
                self.expression(a);
                let left = self.save();
                self.expression(b);
                let right = self.save();
                let operation = match comparison {
                    Comparison::Equal => 7,
                    Comparison::NotEqual => 8,
                    Comparison::Less => 9,
                    Comparison::LessEqual => 10,
                    Comparison::Greater => 11,
                    Comparison::GreaterEqual => 12,
                };
                self.evaluate(operation, a.ty, a.ty, &[left, right]);
                self.next_slot = mark;
            }
        }
    }
    fn instructions(&mut self, instructions: &[Instruction], function: &Function) {
        for instruction in instructions {
            let mark = self.next_slot;
            match instruction {
                Instruction::Noop => (),
                Instruction::Assign(slot, value) => {
                    self.expression(value);
                    self.clone_class(value.ty);
                    self.store(*slot);
                }
                Instruction::Disconnect(destination, source) => {
                    self.load(*source);
                    self.store(*destination);
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
                Instruction::SetField(object, index, value) => {
                    self.expression(object);
                    let receiver = self.save();
                    self.expression(value);
                    self.clone_class(value.ty);
                    let field_value = self.save();
                    self.load(receiver);
                    self.emit("    mov r11, rax");
                    self.load(field_value);
                    self.emit(format!(
                        "    mov [r11 + {}], rax\n    mov [r11 + {}], rdx",
                        index * 16,
                        index * 16 + 8
                    ));
                }
                Instruction::If(condition, yes, no) => {
                    let else_label = self.label("else");
                    let end = self.label("if_end");
                    self.expression(condition);
                    self.emit(format!("    test rax, rax\n    jz {else_label}"));
                    self.instructions(yes, function);
                    self.emit(format!("    jmp {end}\n{else_label}:"));
                    self.instructions(no, function);
                    self.emit(format!("{end}:"));
                }
                Instruction::While(condition, body) | Instruction::Until(condition, body) => {
                    let start = self.label("condition");
                    let end = self.label("loop_end");
                    self.loop_stack.push((start.clone(), end.clone()));
                    self.emit(format!("{start}:"));
                    self.expression(condition);
                    let jump = if matches!(instruction, Instruction::While(_, _)) {
                        "jz"
                    } else {
                        "jnz"
                    };
                    self.emit(format!("    test rax, rax\n    {jump} {end}"));
                    self.instructions(body, function);
                    self.emit(format!("    jmp {start}\n{end}:"));
                    self.loop_stack.pop();
                }
                Instruction::Loop(body) => {
                    let start = self.label("loop");
                    let end = self.label("loop_end");
                    self.loop_stack.push((start.clone(), end.clone()));
                    self.emit(format!("{start}:"));
                    self.instructions(body, function);
                    self.emit(format!("    jmp {start}\n{end}:"));
                    self.loop_stack.pop();
                }
                Instruction::For(slot, start_value, end_value, body) => {
                    self.expression(start_value);
                    self.store(*slot);
                    self.expression(end_value);
                    let end_slot = self.save();
                    let condition = self.label("for_condition");
                    let increment = self.label("for_increment");
                    let end = self.label("loop_end");
                    self.loop_stack.push((increment.clone(), end.clone()));
                    self.emit(format!("{condition}:"));
                    self.evaluate(9, start_value.ty, start_value.ty, &[*slot, end_slot]);
                    self.emit(format!("    test rax, rax\n    jz {end}"));
                    self.instructions(body, function);
                    self.emit(format!("{increment}:\n    mov rax, 1\n    xor edx, edx"));
                    let one = self.save();
                    self.evaluate(0, start_value.ty, start_value.ty, &[*slot, one]);
                    self.store(*slot);
                    self.emit(format!("    jmp {condition}\n{end}:"));
                    self.loop_stack.pop();
                }
                Instruction::Match(value, arms, fallback) => {
                    self.expression(value);
                    let matched_value = self.save();
                    let end = self.label("match_end");
                    for (pattern, body) in arms {
                        let next = self.label("match_next");
                        self.expression(pattern);
                        let pattern = self.save();
                        self.evaluate(7, value.ty, value.ty, &[matched_value, pattern]);
                        self.emit(format!("    test rax, rax\n    jz {next}"));
                        self.instructions(body, function);
                        self.emit(format!("    jmp {end}\n{next}:"));
                    }
                    if let Some(body) = fallback {
                        self.instructions(body, function);
                    }
                    self.emit(format!("{end}:"));
                }
                Instruction::Break => {
                    self.emit(format!("    jmp {}", self.loop_stack.last().unwrap().1))
                }
                Instruction::Continue => {
                    self.emit(format!("    jmp {}", self.loop_stack.last().unwrap().0))
                }
                Instruction::Return => self.emit(format!("    jmp ad_return_{}", function.name)),
            }
            self.next_slot = mark;
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
        self.instructions(&function.instructions, function);
        self.emit(format!("ad_return_{}:", function.name));
        if let Some(slot) = function.result {
            self.load(slot);
        } else {
            self.emit("    xor eax, eax\n    xor edx, edx");
        }
        self.emit("    mov rsp, rbp\n    pop rbp\n    ret");
        let frame = self.max_slot * 16 + 32;
        self.text.insert_str(start,&format!("ad_fun_{}:\n    push rbp\n    mov rbp, rsp\n    mov r11, {frame}\nad_probe_{}:\n    cmp r11, 4096\n    jb ad_tail_{}\n    sub rsp, 4096\n    test byte [rsp], 0\n    sub r11, 4096\n    jmp ad_probe_{}\nad_tail_{}:\n    sub rsp, r11\n    test byte [rsp], 0\n",function.name,function.name,function.name,function.name,function.name));
    }
    fn call_saved(&mut self, name: &str, slots: &[usize]) {
        let size = slots.len() * 16;
        if size != 0 {
            self.emit(format!("    sub rsp, {size}"));
        }
        for (index, slot) in slots.iter().enumerate() {
            self.load(*slot);
            self.emit(format!(
                "    mov [rsp + {}], rax\n    mov [rsp + {}], rdx",
                index * 16,
                index * 16 + 8
            ));
        }
        self.emit(format!("    call ad_fun_{name}"));
        if size != 0 {
            self.emit(format!("    add rsp, {size}"));
        }
    }
}
pub fn assembly(program: &Program) -> String {
    let mut generator = Generator {
        text: include_str!("runtime.asm").into(),
        data: Vec::new(),
        next_slot: 0,
        max_slot: 0,
        class_sizes: program.class_sizes.clone(),
        next_label: 0,
        loop_stack: Vec::new(),
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
