use rustc_apfloat::{
    Float, FloatConvert, Round, Status,
    ieee::{Double, Quad, Single},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    U4,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    F128,
    String,
    Bool,
    None,
    Enum(u32),
    Class(u32),
}

impl Type {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" | "int" => Self::I32,
            "i64" => Self::I64,
            "u4" => Self::U4,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" | "u" => Self::U32,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" | "float" => Self::F64,
            "f128" => Self::F128,
            "string" => Self::String,
            "bool" => Self::Bool,
            "None" => Self::None,
            _ => return None,
        })
    }
    pub fn from_id(id: u32) -> Option<Self> {
        if id & 0x8000_0000 != 0 {
            return Some(Self::Enum(id & 0x7fff_ffff));
        }
        if id & 0x4000_0000 != 0 {
            return Some(Self::Class(id & 0x3fff_ffff));
        }
        [
            Self::I8,
            Self::I16,
            Self::I32,
            Self::I64,
            Self::U4,
            Self::U8,
            Self::U16,
            Self::U32,
            Self::U64,
            Self::F32,
            Self::F64,
            Self::F128,
            Self::String,
            Self::Bool,
            Self::None,
        ]
        .get(id as usize)
        .copied()
    }
    pub fn id(self) -> u32 {
        match self {
            Self::I8 => 0,
            Self::I16 => 1,
            Self::I32 => 2,
            Self::I64 => 3,
            Self::U4 => 4,
            Self::U8 => 5,
            Self::U16 => 6,
            Self::U32 => 7,
            Self::U64 => 8,
            Self::F32 => 9,
            Self::F64 => 10,
            Self::F128 => 11,
            Self::String => 12,
            Self::Bool => 13,
            Self::None => 14,
            Self::Enum(id) => 0x8000_0000 | id,
            Self::Class(id) => 0x4000_0000 | id,
        }
    }
    pub fn integer(self) -> bool {
        matches!(
            self,
            Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::U4
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
        )
    }
    pub fn floating(self) -> bool {
        matches!(self, Self::F32 | Self::F64 | Self::F128)
    }
    pub fn numeric(self) -> bool {
        self.integer() || self.floating()
    }
    pub fn unsigned(self) -> bool {
        matches!(
            self,
            Self::U4 | Self::U8 | Self::U16 | Self::U32 | Self::U64
        )
    }
    pub fn bounds(self) -> (i128, i128) {
        match self {
            Self::I8 => (i8::MIN as i128, i8::MAX as i128),
            Self::I16 => (i16::MIN as i128, i16::MAX as i128),
            Self::I32 => (i32::MIN as i128, i32::MAX as i128),
            Self::I64 => (i64::MIN as i128, i64::MAX as i128),
            Self::U4 => (0, 15),
            Self::U8 => (0, u8::MAX as i128),
            Self::U16 => (0, u16::MAX as i128),
            Self::U32 => (0, u32::MAX as i128),
            Self::U64 => (0, u64::MAX as i128),
            _ => (0, 0),
        }
    }
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U4 => "u4",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::F128 => "f128",
            Self::String => "string",
            Self::Bool => "bool",
            Self::None => "None",
            Self::Enum(_) => "enum",
            Self::Class(_) => "class",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Value {
    pub lo: u64,
    pub hi: u64,
}
impl Value {
    pub fn bits(self) -> u128 {
        (self.hi as u128) << 64 | self.lo as u128
    }
    pub fn from_bits(bits: u128) -> Self {
        Self {
            lo: bits as u64,
            hi: (bits >> 64) as u64,
        }
    }
    pub fn integer(self, ty: Type) -> i128 {
        if ty.unsigned() {
            self.lo as i128
        } else {
            self.lo as i64 as i128
        }
    }
}

pub fn integer(value: i128, ty: Type) -> Result<Value, String> {
    let (low, high) = ty.bounds();
    if !ty.integer() || value < low || value > high {
        return Err(format!("value {value} is outside the range of {ty}"));
    }
    Ok(Value {
        lo: value as u64,
        hi: 0,
    })
}
fn float_literal<F: Float>(text: &str) -> Result<Value, String> {
    let result = F::from_str_r(text, Round::NearestTiesToEven)
        .map_err(|e| format!("invalid floating-point literal: {e:?}"))?;
    if result
        .status
        .intersects(Status::OVERFLOW | Status::INVALID_OP)
        || !result.value.is_finite()
    {
        return Err("floating-point literal is outside the finite range".into());
    }
    Ok(Value::from_bits(result.value.to_bits()))
}
pub fn literal(text: &str, ty: Type) -> Result<Value, String> {
    if ty.integer() {
        return integer(
            text.parse::<i128>()
                .map_err(|_| format!("invalid integer literal '{text}' for {ty}"))?,
            ty,
        );
    }
    match ty {
        Type::F32 => float_literal::<Single>(text),
        Type::F64 => float_literal::<Double>(text),
        Type::F128 => float_literal::<Quad>(text),
        _ => Err(format!("numeric literal cannot have type {ty}")),
    }
}

fn floating<F: Float>(op: u32, a: Value, b: Value, c: Value) -> Result<Value, String> {
    let (a, b, c) = (
        F::from_bits(a.bits()),
        F::from_bits(b.bits()),
        F::from_bits(c.bits()),
    );
    if (7..=12).contains(&op) {
        let result = match op {
            7 => a == b,
            8 => a != b,
            9 => a < b,
            10 => a <= b,
            11 => a > b,
            12 => a >= b,
            _ => unreachable!(),
        };
        return Ok(Value {
            lo: result as u64,
            hi: 0,
        });
    }
    let result = match op {
        0 => a + b,
        1 => a - b,
        2 => a * b,
        3 => a / b,
        4 => return Ok(Value::from_bits((-a).to_bits())),
        6 => {
            if b > c {
                return Err("invalid clamp range".into());
            }
            return Ok(Value::from_bits(
                if a < b {
                    b
                } else if a > c {
                    c
                } else {
                    a
                }
                .to_bits(),
            ));
        }
        _ => return Err("invalid operation".into()),
    };
    if result
        .status
        .intersects(Status::OVERFLOW | Status::INVALID_OP | Status::DIV_BY_ZERO)
        || !result.value.is_finite()
    {
        return Err("floating-point overflow or division by zero".into());
    }
    Ok(Value::from_bits(result.value.to_bits()))
}
pub fn operation(op: u32, ty: Type, a: Value, b: Value, c: Value) -> Result<Value, String> {
    if ty.integer() {
        let (a, b, c) = (a.integer(ty), b.integer(ty), c.integer(ty));
        if (7..=12).contains(&op) {
            let result = match op {
                7 => a == b,
                8 => a != b,
                9 => a < b,
                10 => a <= b,
                11 => a > b,
                12 => a >= b,
                _ => unreachable!(),
            };
            return Ok(Value {
                lo: result as u64,
                hi: 0,
            });
        }
        let value = match op {
            0 => a.checked_add(b),
            1 => a.checked_sub(b),
            2 => a.checked_mul(b),
            3 => a.checked_div(b),
            4 => a.checked_neg(),
            6 if b <= c => Some(a.clamp(b, c)),
            _ => None,
        }
        .ok_or("arithmetic overflow, division by zero or invalid clamp range")?;
        return integer(value, ty);
    }
    if matches!(ty, Type::Bool | Type::None | Type::Enum(_)) && matches!(op, 7 | 8) {
        return Ok(Value {
            lo: (if op == 7 { a == b } else { a != b }) as u64,
            hi: 0,
        });
    }
    match ty {
        Type::F32 => floating::<Single>(op, a, b, c),
        Type::F64 => floating::<Double>(op, a, b, c),
        Type::F128 => floating::<Quad>(op, a, b, c),
        _ => Err(format!("arithmetic is not supported for {ty}")),
    }
}
pub fn convert(value: Value, from: Type, to: Type) -> Result<Value, String> {
    if from == to {
        return Ok(value);
    }
    if from.integer() && to.integer() {
        return integer(value.integer(from), to);
    }
    if from.integer() && to.floating() {
        return literal(&value.integer(from).to_string(), to);
    }
    if from.floating() && to.floating() {
        return match (from, to) {
            (Type::F32, Type::F64) => float_convert::<Single, Double>(value),
            (Type::F32, Type::F128) => float_convert::<Single, Quad>(value),
            (Type::F64, Type::F32) => float_convert::<Double, Single>(value),
            (Type::F64, Type::F128) => float_convert::<Double, Quad>(value),
            (Type::F128, Type::F32) => float_convert::<Quad, Single>(value),
            (Type::F128, Type::F64) => float_convert::<Quad, Double>(value),
            _ => unreachable!(),
        };
    }
    Err(format!("cannot convert {from} to {to}"))
}
fn float_convert<F: Float + FloatConvert<T>, T: Float>(value: Value) -> Result<Value, String> {
    let result = F::from_bits(value.bits()).convert(&mut false);
    if result
        .status
        .intersects(Status::OVERFLOW | Status::INVALID_OP)
    {
        return Err("floating-point conversion overflow".into());
    }
    Ok(Value::from_bits(result.value.to_bits()))
}
pub fn display(value: Value, ty: Type) -> Result<String, String> {
    if ty.integer() {
        return Ok(value.integer(ty).to_string());
    }
    Ok(match ty {
        Type::F32 => f32::from_bits(value.lo as u32).to_string(),
        Type::F64 => f64::from_bits(value.lo).to_string(),
        Type::F128 => Quad::from_bits(value.bits()).to_string(),
        Type::Bool => if value.lo == 0 { "false" } else { "true" }.into(),
        Type::None => "None".into(),
        Type::Enum(_) => value.lo.to_string(),
        Type::Class(_) => return Err("class objects cannot be printed directly".into()),
        _ => return Err("string values must be written as UTF-8 bytes".into()),
    })
}
