use crate::{
    syntax::{Comparison, LogicalOperator, Operator},
    typed::{ClassInfo, Expression, Function, Instruction, Kind, Program},
    types::Type,
};

struct Generator {
    text: String,
    data: Vec<Vec<u8>>,
    next_slot: usize,
    max_slot: usize,
    class_sizes: Vec<usize>,
    classes: Vec<ClassInfo>,
    next_label: usize,
    loop_stack: Vec<(String, String)>,
    error_targets: Vec<String>,
    current_function_name: String,
    current_function_types: Vec<Type>,
}
fn memory(slot: usize, offset: usize) -> String {
    format!("[rbp - {}]", (slot + 1) * 16 - offset)
}
impl Generator {
    fn error_target(&self) -> &str {
        self.error_targets
            .last()
            .expect("code generation requires an error target")
    }
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
    fn print_loaded(&mut self, ty: Type, newline: bool) {
        let slot = self.save();
        let error = self.error_target().to_string();
        self.emit(format!(
            "    lea rcx, {}\n    mov edx, {}\n    mov r8d, {}\n    call ad_print\n    test eax, eax\n    jnz {error}",
            memory(slot, 0),
            ty.id(),
            u8::from(newline)
        ));
    }
    fn print_text(&mut self, text: &str, newline: bool) {
        let index = self.data.len();
        self.data.push(text.as_bytes().to_vec());
        self.emit(format!(
            "    lea rax, [rel ad_string_{index}]\n    mov rdx, {}",
            text.len()
        ));
        self.print_loaded(Type::String, newline);
    }
    fn print_class(&mut self, value: &Expression, id: u32, newline: bool) {
        self.expression(value);
        let object = self.save();
        let class = self.classes[id as usize].clone();
        self.print_text(&format!("{}(", class.name), false);
        let mut first = true;
        for (index, field) in class.fields.iter().enumerate() {
            if !field.public {
                continue;
            }
            if !first {
                self.print_text(", ", false);
            }
            first = false;
            self.print_text(&format!("{}=", field.name), false);
            self.load(object);
            self.emit("    mov r11, rax");
            self.emit(format!(
                "    mov rax, [r11 + {}]\n    mov rdx, [r11 + {}]",
                index * 16,
                index * 16 + 8
            ));
            self.print_loaded(field.ty, false);
        }
        self.print_text(")", newline);
    }
    fn clone_class(&mut self, ty: Type) {
        match ty {
            Type::Class(id) => self.emit(format!(
                "    mov rcx, rax\n    mov edx, {}\n    call ad_object_clone\n    xor edx, edx",
                self.class_sizes[id as usize]
            )),
            Type::List(_) => self.emit(
                "    sub rsp, 16\n    mov [rsp], rdx\n    mov rcx, rax\n    call ad_object_clone\n    mov rdx, [rsp]\n    add rsp, 16",
            ),
            _ => (),
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
        let returned = self.save();
        let error = self.error_target().to_string();
        self.emit(format!(
            "    call ad_has_error\n    test eax, eax\n    jnz {error}"
        ));
        self.load(returned);
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
        let error = self.error_target().to_string();
        self.emit(format!("    mov dword {}, {operation}\n    mov dword {}, {}\n    mov dword {}, {}\n    lea rcx, {}\n    call ad_evaluate\n    test eax, eax\n    jnz {error}\n    mov rax, {}\n    mov rdx, {}", memory(request,64), memory(request,68),ty.id(),memory(request,72),from.id(),memory(request,0),memory(request,48),memory(request,56)));
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
            Kind::Try(instructions) => {
                let result = self.reserve(1);
                let failed = self.label("try_failed");
                let done = self.label("try_done");
                self.emit("    call ad_try_begin");
                self.error_targets.push(failed.clone());
                let context = Function {
                    name: self.current_function_name.clone(),
                    parameters: 0,
                    result: None,
                    types: self.current_function_types.clone(),
                    instructions: Vec::new(),
                    parameter_names: Vec::new(),
                };
                self.instructions(instructions, &context);
                self.error_targets.pop();
                self.emit(format!(
                    "    lea rcx, {}\n    call ad_try_end\n    jmp {done}\n{failed}:\n    lea rcx, {}\n    call ad_try_end\n{done}:\n    mov rax, {}\n    mov rdx, {}",
                    memory(result, 0),
                    memory(result, 0),
                    memory(result, 0),
                    memory(result, 8)
                ));
            }
            Kind::Address(slot) => {
                self.emit(format!(
                    "    lea rax, {}\n    xor edx, edx",
                    memory(*slot, 0)
                ));
            }
            Kind::Dereference(value) => {
                self.expression(value);
                self.emit("    mov r11, rax\n    mov rax, [r11]\n    mov rdx, [r11 + 8]");
            }
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
            Kind::List(values) => {
                let mark = self.next_slot;
                let mut elements = Vec::new();
                for value in values {
                    self.expression(value);
                    self.clone_class(value.ty);
                    elements.push(self.save());
                }
                self.emit(format!(
                    "    mov ecx, {}\n    call ad_object_new",
                    values.len()
                ));
                let list = self.save();
                for (index, element) in elements.iter().enumerate() {
                    self.load(list);
                    self.emit("    mov r11, rax");
                    self.load(*element);
                    self.emit(format!(
                        "    mov [r11 + {}], rax\n    mov [r11 + {}], rdx",
                        index * 16,
                        index * 16 + 8
                    ));
                }
                self.load(list);
                self.emit(format!("    mov rdx, {}", values.len()));
                self.next_slot = mark;
            }
            Kind::Index(list, index) => {
                let mark = self.next_slot;
                self.expression(list);
                let list = self.save();
                self.expression(index);
                let index = self.save();
                self.emit_list_bounds(list, index);
                self.load(list);
                self.emit("    mov r11, rax");
                self.load(index);
                self.emit(
                    "    shl rax, 4\n    add r11, rax\n    mov rax, [r11]\n    mov rdx, [r11 + 8]",
                );
                self.next_slot = mark;
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
            Kind::Unwrap(value) => {
                let compact = self.label("optional_compact");
                let done = self.label("optional_unwrapped");
                self.expression(value);
                let error = self.error_target().to_string();
                self.emit(format!("    test rdx, rdx\n    jnz {compact}\n    call ad_optional_error\n    jmp {error}\n{compact}:\n    cmp rdx, 2\n    jne {done}\n    mov r11, rax\n    mov rax, [r11]\n    mov rdx, [r11 + 8]\n{done}:"));
            }
            Kind::Not(value) => {
                self.expression(value);
                self.emit("    test rax, rax\n    sete al\n    movzx eax, al\n    xor edx, edx");
            }
            Kind::Logical(operator, a, b) => {
                let skip = self.label("logical_skip");
                let end = self.label("logical_end");
                self.expression(a);
                self.emit("    test rax, rax");
                match operator {
                    LogicalOperator::And => self.emit(format!("    jz {skip}")),
                    LogicalOperator::Or => self.emit(format!("    jnz {skip}")),
                }
                self.expression(b);
                self.emit(format!(
                    "    jmp {end}\n{skip}:\n    mov eax, {}\n    xor edx, edx\n{end}:",
                    u8::from(matches!(operator, LogicalOperator::Or))
                ));
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
                    Operator::Remainder => 13,
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
                Instruction::Remove(object, hook) => {
                    if let Some(hook) = hook {
                        self.expression(object);
                        let receiver = self.save();
                        self.call_saved(hook, &[receiver]);
                    }
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
                    if let Type::Class(id) = value.ty {
                        self.print_class(value, id, *newline);
                    } else {
                        self.expression(value);
                        self.print_loaded(value.ty, *newline);
                    }
                }
                Instruction::Message(message, panic, line) => {
                    self.expression(message);
                    let message = self.save();
                    self.emit(format!(
                        "    lea rcx, {}\n    mov edx, {}\n    mov r8d, {}\n    call ad_message",
                        memory(message, 0),
                        line,
                        u8::from(*panic)
                    ));
                    if *panic {
                        let error = self.error_target().to_string();
                        self.emit(format!("    jmp {error}"));
                    }
                }
                Instruction::Call(expr) => self.expression(expr),
                Instruction::Exit => self.emit("    xor ecx, ecx\n    call ExitProcess"),
                Instruction::SetField(object, index, value, hook) => {
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
                    if let Some(hook) = hook {
                        self.call_saved(hook, &[receiver]);
                    }
                }
                Instruction::SetIndex(list, index, value) => {
                    self.expression(list);
                    let list = self.save();
                    self.expression(index);
                    let index = self.save();
                    self.expression(value);
                    self.clone_class(value.ty);
                    let value = self.save();
                    self.emit_list_bounds(list, index);
                    self.load(list);
                    self.emit("    mov r11, rax");
                    self.load(index);
                    self.emit("    shl rax, 4\n    add r11, rax");
                    self.load(value);
                    self.emit("    mov [r11], rax\n    mov [r11 + 8], rdx");
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
                Instruction::ForEach(slot, collection, body) => {
                    let Type::List(inner) = collection.ty else {
                        unreachable!("typed for-each collection must be a List")
                    };
                    let element_ty = Type::from_id(inner).expect("checked List element type");
                    self.expression(collection);
                    let list = self.save();
                    self.emit("    xor eax, eax\n    xor edx, edx");
                    let index = self.save();
                    let condition = self.label("for_each_condition");
                    let increment = self.label("for_each_increment");
                    let end = self.label("loop_end");
                    self.loop_stack.push((increment.clone(), end.clone()));
                    self.emit(format!("{condition}:"));
                    self.load(index);
                    self.emit(format!("    cmp rax, {}\n    jae {end}", memory(list, 8)));
                    self.load(list);
                    self.emit("    mov r11, rax");
                    self.load(index);
                    self.emit(
                        "    shl rax, 4\n    add r11, rax\n    mov rax, [r11]\n    mov rdx, [r11 + 8]",
                    );
                    self.clone_class(element_ty);
                    self.store(*slot);
                    self.instructions(body, function);
                    self.emit(format!(
                        "{increment}:\n    mov rax, {}\n    inc rax\n    xor edx, edx",
                        memory(index, 0)
                    ));
                    self.store(index);
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
        self.current_function_name.clone_from(&function.name);
        self.current_function_types.clone_from(&function.types);
        self.next_slot = function.types.len();
        self.max_slot = self.next_slot;
        let start = self.text.len();
        let error = format!("ad_error_{}", function.name);
        self.error_targets.push(error.clone());
        for slot in 0..function.parameters {
            self.emit(format!(
                "    mov rax, [rbp + {}]\n    mov rdx, [rbp + {}]",
                16 + slot * 16,
                24 + slot * 16
            ));
            self.store(slot);
        }
        self.instructions(&function.instructions, function);
        self.error_targets.pop();
        self.emit(format!(
            "    jmp ad_return_{}\n{error}:\n    call ad_is_trying\n    test eax, eax\n    jnz ad_return_{}\n    mov ecx, 2\n    call ExitProcess",
            function.name,
            function.name
        ));
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
        let returned = self.save();
        let error = self.error_target().to_string();
        self.emit(format!(
            "    call ad_has_error\n    test eax, eax\n    jnz {error}"
        ));
        self.load(returned);
    }
    fn emit_list_bounds(&mut self, list: usize, index: usize) {
        let valid = self.label("list_index_valid");
        self.load(index);
        self.emit(format!("    cmp rax, {}\n    jb {valid}", memory(list, 8)));
        let error = self.error_target().to_string();
        self.emit(format!(
            "    mov rcx, rax\n    mov rdx, {}\n    call ad_list_error\n    jmp {error}\n{valid}:",
            memory(list, 8)
        ));
    }
}
fn cli_entry(function: &Function) -> String {
    let count = function.parameters;
    let output_size = count.max(1) * 16;
    let frame = (output_size + 63) / 16 * 16;
    let mut text = format!(
        "ad_cli_main:\n    push rbp\n    mov rbp, rsp\n    sub rsp, {frame}\n    lea r8, [rel ad_argument_specs]\n    mov r9d, {count}\n    lea rax, [rbp - {output_size}]\n    mov [rsp + 32], rax\n    call ad_parse_arguments\n    test eax, eax\n    jnz ad_exit_error\n"
    );
    if count != 0 {
        text.push_str(&format!("    sub rsp, {}\n", count * 16));
        for index in 0..count {
            text.push_str(&format!(
                "    mov rax, [rbp - {}]\n    mov rdx, [rbp - {}]\n    mov [rsp + {}], rax\n    mov [rsp + {}], rdx\n",
                output_size - index * 16,
                output_size - index * 16 - 8,
                index * 16,
                index * 16 + 8
            ));
        }
    }
    text.push_str(&format!("    call ad_fun_{}\n", function.name));
    if count != 0 {
        text.push_str(&format!("    add rsp, {}\n", count * 16));
    }
    text.push_str("    mov rsp, rbp\n    pop rbp\n    ret\n");
    text
}

pub fn assembly_entry(program: &Program, entry: &str) -> String {
    let runtime = if cfg!(target_os = "linux") {
        include_str!("runtime-linux.asm")
    } else {
        include_str!("runtime.asm")
    };
    let runtime_length = runtime.len();
    let mut generator = Generator {
        text: runtime.into(),
        data: Vec::new(),
        next_slot: 0,
        max_slot: 0,
        class_sizes: program.class_sizes.clone(),
        classes: program.classes.clone(),
        next_label: 0,
        loop_stack: Vec::new(),
        error_targets: Vec::new(),
        current_function_name: String::new(),
        current_function_types: Vec::new(),
    };
    let main = program
        .functions
        .iter()
        .find(|function| function.name == entry)
        .expect("checked entry function must exist");
    generator.emit(cli_entry(main));
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
    generator.text.push_str("ad_argument_specs:\n");
    if main.parameters == 0 {
        generator.text.push_str("    dq 0\n");
    }
    for (index, (name, ty)) in main
        .parameter_names
        .iter()
        .zip(&main.types[..main.parameters])
        .enumerate()
    {
        generator.text.push_str(&format!(
            "    dq ad_argument_name_{index}\n    dq {}\n    dd {}\n    dd {}\n",
            name.len(),
            ty.id(),
            u8::from(matches!(ty, Type::Optional(_)))
        ));
    }
    for (index, name) in main.parameter_names.iter().enumerate() {
        let bytes = name
            .bytes()
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>();
        generator.text.push_str(&format!(
            "ad_argument_name_{index}:\n    db {}\n",
            bytes.join(", ")
        ));
    }
    if cfg!(target_os = "linux") {
        let mut generated = generator.text.split_off(runtime_length);
        for (windows_name, linux_name) in [
            ("ExitProcess", "ad_linux_exit"),
            ("ad_evaluate", "ad_linux_evaluate"),
            ("ad_print", "ad_linux_print"),
            ("ad_message", "ad_linux_message"),
            ("ad_object_new", "ad_linux_object_new"),
            ("ad_object_clone", "ad_linux_object_clone"),
            ("ad_list_error", "ad_linux_list_error"),
            ("ad_optional_error", "ad_linux_optional_error"),
            ("ad_parse_arguments", "ad_linux_parse_arguments"),
            ("ad_try_begin", "ad_linux_try_begin"),
            ("ad_try_end", "ad_linux_try_end"),
            ("ad_has_error", "ad_linux_has_error"),
            ("ad_is_trying", "ad_linux_is_trying"),
        ] {
            generated = generated.replace(
                &format!("call {windows_name}"),
                &format!("call {linux_name}"),
            );
        }
        generator.text.push_str(&generated);
    }
    generator.text
}
