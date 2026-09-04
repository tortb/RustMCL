//! Minecraft `servers.dat`(Java 序列化)解析 —— 模块 1 的批量导入迁移支持。
//!
//! 仅实现解析 `HashMap<String, ServerData>` 所需的 Java ObjectInputStream 子集:
//! 流头 / classDesc / object / String / 基础类型 / 引用 / blockdata。
//! 解析失败时返回错误(前端提示无法导入),绝不 panic。
//!
//! 目录:`core/servers_import/mod.rs`,纯逻辑,便于单元测试。

use crate::error::RmclError;
use serde::Serialize;

const STREAM_MAGIC: [u8; 4] = [0xAC, 0xED, 0x00, 0x05];

const TC_NULL: u8 = 0x70;
const TC_REFERENCE: u8 = 0x71;
const TC_CLASSDESC: u8 = 0x72;
const TC_OBJECT: u8 = 0x73;
const TC_STRING: u8 = 0x74;
const TC_ARRAY: u8 = 0x75;
const TC_BLOCKDATA: u8 = 0x77;
const TC_ENDBLOCKDATA: u8 = 0x78;
const TC_RESET: u8 = 0x79;
const TC_BLOCKDATALONG: u8 = 0x7A;
const TC_LONGSTRING: u8 = 0x7C;
const TC_ENUM: u8 = 0x7E;

/// 一条导入的服务器记录
#[derive(Debug, Clone, Serialize)]
pub struct ImportedServer {
    pub name: String,
    pub address: String,
    pub port: u16,
}

/// classDesc 里的一个字段
#[derive(Debug, Clone)]
enum FieldKind {
    /// 对象类型(以引用形式出现,读取时走 read_object)
    Obj,
    /// 基础类型,type_code 为 Java 内部的 B/C/D/F/I/J/S/Z
    Prim(u8),
}

#[derive(Debug, Clone)]
struct FieldDef {
    name: String,
    kind: FieldKind,
}

#[derive(Debug, Clone)]
struct ClassDesc {
    name: String,
    fields: Vec<FieldDef>,
}

/// 解析值(最小集合,主要用于把手柄表编号对齐)
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Value {
    Null,
    Str(String),
    Int(i64),
    Bool(bool),
    Obj(Vec<(String, Value)>),
    ClassDesc(ClassDesc),
    Blob(Vec<u8>),
    Arr(Vec<Value>),
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    handles: Vec<Value>,
}

/// 从 servers.dat 二进制解析服务器列表
pub fn parse_servers(raw: &[u8]) -> Result<Vec<ImportedServer>, RmclError> {
    let mut r = Reader {
        data: raw,
        pos: 0,
        handles: Vec::new(),
    };
    r.read_stream_header()?;
    let value = r.read_object()?;
    r.collect_servers(&value)
}

impl<'a> Reader<'a> {
    fn read_stream_header(&mut self) -> Result<(), RmclError> {
        if self.data.len() < 4 || self.data[..4] != STREAM_MAGIC {
            return Err(RmclError::other("不是有效的 servers.dat 序列化数据"));
        }
        self.pos = 4;
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, RmclError> {
        let b = *self
            .data
            .get(self.pos)
            .ok_or_else(|| RmclError::other("servers.dat 数据截断"))?;
        self.pos += 1;
        Ok(b)
    }

    fn read_u16(&mut self) -> Result<u16, RmclError> {
        let hi = self.read_u8()? as u16;
        let lo = self.read_u8()? as u16;
        Ok((hi << 8) | lo)
    }

    fn read_u32(&mut self) -> Result<u32, RmclError> {
        let a = self.read_u8()? as u32;
        let b = self.read_u8()? as u32;
        let c = self.read_u8()? as u32;
        let d = self.read_u8()? as u32;
        Ok((a << 24) | (b << 16) | (c << 8) | d)
    }

    fn read_i32(&mut self) -> Result<i32, RmclError> {
        Ok(self.read_u32()? as i32)
    }

    fn read_i64(&mut self) -> Result<i64, RmclError> {
        let hi = self.read_u32()? as i64;
        let lo = self.read_u32()? as i64;
        Ok((hi << 32) | lo)
    }

    // 读取 Java modified-UTF-8 字符串(TC_STRING/TC_LONGSTRING 的内容)
    fn read_modified_utf(&mut self, len: usize) -> Result<String, RmclError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| RmclError::other("servers.dat 长度溢出"))?;
        let bytes = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| RmclError::other("servers.dat 字符串截断"))?;
        self.pos = end;
        // 解码 modified UTF-8:0x0000 -> C0 80;0x8000+ 用三字节编码。
        // 此处采用宽松解码:先尝试标准 UTF-8,失败回退为逐字节 0x00-0xFF 转拉丁近似。
        let mut out = String::new();
        let mut i = 0;
        let mut utf8 = Vec::new();
        while i < bytes.len() {
            let b = bytes[i];
            if b & 0x80 == 0 {
                // 单字节
                utf8.push(if b == 0 { b'?' } else { b });
                i += 1;
            } else if b & 0xE0 == 0xC0 {
                // 两字节:C0 80 表示 U+0000
                if bytes.get(i + 1).is_some() {
                    let c0 = bytes[i] & 0x1F;
                    let c1 = bytes[i + 1] & 0x3F;
                    let code = ((c0 as u32) << 6) | c1 as u32;
                    push_char(&mut utf8, code);
                    i += 2;
                } else {
                    utf8.push(b'?');
                    i += 1;
                }
            } else if b & 0xF0 == 0xE0 {
                if bytes.get(i + 2).is_some() {
                    let c0 = bytes[i] & 0x0F;
                    let c1 = bytes[i + 1] & 0x3F;
                    let c2 = bytes[i + 2] & 0x3F;
                    let code = ((c0 as u32) << 12) | ((c1 as u32) << 6) | c2 as u32;
                    push_char(&mut utf8, code);
                    i += 3;
                } else {
                    utf8.push(b'?');
                    i += 2;
                }
            } else {
                utf8.push(b'?');
                i += 1;
            }
        }
        out.push_str(&String::from_utf8_lossy(&utf8));
        Ok(out)
    }

    /// 读取一个完整对象(分配句柄,与 Java 对象流句柄编号对齐)
    fn read_object(&mut self) -> Result<Value, RmclError> {
        let tag = self.read_u8()?;
        match tag {
            TC_NULL => Ok(Value::Null),
            TC_REFERENCE => {
                let h = self.read_u32()? as usize;
                Ok(self.handles.get(h).cloned().unwrap_or(Value::Null))
            }
            TC_STRING => {
                let len = self.read_u16()? as usize;
                let s = self.read_modified_utf(len)?;
                self.handles.push(Value::Str(s.clone()));
                Ok(Value::Str(s))
            }
            TC_LONGSTRING => {
                let len = self.read_u32()? as usize;
                let s = self.read_modified_utf(len)?;
                self.handles.push(Value::Str(s.clone()));
                Ok(Value::Str(s))
            }
            TC_OBJECT => self.read_object_instance(),
            TC_ARRAY => {
                let comp = self.read_class_desc()?;
                let len = self.read_i32()?;
                let mut items = Vec::new();
                if let Some(desc) = comp {
                    if desc.fields.is_empty() {
                        // 基元数组:字段名即类型码,用 name 首字符串近似;按 1 字节跳过
                        let size = self.array_cell_size(&desc.name)?;
                        let bytes = size * len.max(0) as usize;
                        self.pos = (self.pos + bytes).min(self.data.len());
                        items.push(Value::Blob(vec![]));
                    } else {
                        for _ in 0..len.max(0) {
                            items.push(self.read_object()?);
                        }
                    }
                }
                self.handles.push(Value::Arr(items.clone()));
                Ok(Value::Arr(items))
            }
            TC_ENUM => {
                // enum:classDesc + 常量名;跳过
                let _ = self.read_class_desc()?;
                let _ = self.read_object()?;
                Ok(Value::Blob(vec![]))
            }
            // blockdata 直接跳过其内容
            TC_BLOCKDATA => {
                let len = self.read_u8()? as usize;
                self.skip(len)?;
                Ok(Value::Blob(vec![]))
            }
            TC_BLOCKDATALONG => {
                let len = self.read_u32()? as usize;
                self.skip(len)?;
                Ok(Value::Blob(vec![]))
            }
            TC_RESET => {
                self.handles.clear();
                Ok(Value::Blob(vec![]))
            }
            _ => Err(RmclError::other(format!(
                "不支持的序列化类型标记 0x{tag:02x}"
            ))),
        }
    }

    fn array_cell_size(&self, comp_name: &str) -> Result<usize, RmclError> {
        Ok(match comp_name {
            "B" | "Z" => 1,
            "S" | "C" => 2,
            "I" | "F" => 4,
            "J" | "D" => 8,
            _ => return Err(RmclError::other("未知数组元素类型")),
        })
    }

    fn skip(&mut self, len: usize) -> Result<(), RmclError> {
        self.pos = self
            .pos
            .checked_add(len)
            .filter(|&n| n <= self.data.len())
            .ok_or_else(|| RmclError::other("servers.dat 数据越界"))?;
        Ok(())
    }

    /// 读取类描述;TC_REFERENCE 时从句柄表取回,TC_NULL 返回 None
    fn read_class_desc(&mut self) -> Result<Option<ClassDesc>, RmclError> {
        let tag = self.read_u8()?;
        match tag {
            TC_NULL => Ok(None),
            TC_REFERENCE => {
                let h = self.read_u32()? as usize;
                match self.handles.get(h) {
                    Some(Value::ClassDesc(d)) => Ok(Some(d.clone())),
                    _ => Err(RmclError::other("类描述引用无效")),
                }
            }
            TC_CLASSDESC => {
                // 先占句柄
                let slot = self.handles.len();
                self.handles.push(Value::ClassDesc(ClassDesc {
                    name: String::new(),
                    fields: Vec::new(),
                }));
                let name_len = self.read_u16()? as usize;
                let name = self.read_modified_utf(name_len)?;
                let _serial_uid = self.read_i64()?;
                let _flags = self.read_u8()?;
                let field_count = self.read_u16()? as usize;
                let mut fields = Vec::new();
                for _ in 0..field_count {
                    let fname_len = self.read_u16()? as usize;
                    let fname = self.read_modified_utf(fname_len)?;
                    let typecode = self.read_u8()? as char;
                    let kind = match typecode {
                        'L' | '[' => {
                            // 对象类型:再跟一个类名字符串
                            let cls_len = self.read_u16()? as usize;
                            let _cls = self.read_modified_utf(cls_len)?;
                            FieldKind::Obj
                        }
                        c => FieldKind::Prim(c as u8),
                    };
                    fields.push(FieldDef { name: fname, kind });
                }
                // 类注解:读到 TC_ENDBLOCKDATA(通常无内容)
                loop {
                    let b = self.peek_u8()?;
                    if b == TC_ENDBLOCKDATA {
                        self.read_u8()?;
                        break;
                    }
                    self.read_object()?;
                }
                // super 类描述(通常为 TC_NULL)
                let _super = self.read_class_desc()?;
                let desc = ClassDesc { name, fields };
                self.handles[slot] = Value::ClassDesc(desc.clone());
                Ok(Some(desc))
            }
            _ => Err(RmclError::other("预期类描述标记")),
        }
    }

    fn peek_u8(&mut self) -> Result<u8, RmclError> {
        self.data
            .get(self.pos)
            .copied()
            .ok_or_else(|| RmclError::other("servers.dat 数据截断"))
    }

    fn read_object_instance(&mut self) -> Result<Value, RmclError> {
        let desc = self.read_class_desc()?;
        let Some(desc) = desc else {
            return Ok(Value::Null);
        };
        // 对象自身占一个句柄;字段值(对象/字符串)随读随分配
        let slot = self.handles.len();
        self.handles.push(Value::Obj(Vec::new()));
        let mut obj = Vec::new();
        if desc.name.contains("HashMap") || desc.name.contains("Hashtable") {
            // Map:defaultWriteObject 无持久字段,内容为 size + (key,value) 对
            let size = self.read_i32()?;
            for _ in 0..size.max(0) {
                let _key = self.read_object()?;
                let val = self.read_object()?;
                obj.push((String::new(), val));
            }
        } else {
            for f in &desc.fields {
                let v = match f.kind {
                    FieldKind::Obj => self.read_object()?,
                    FieldKind::Prim(code) => self.read_prim(code)?,
                };
                obj.push((f.name.clone(), v));
            }
        }
        self.handles[slot] = Value::Obj(obj.clone());
        Ok(Value::Obj(obj))
    }

    fn read_prim(&mut self, code: u8) -> Result<Value, RmclError> {
        Ok(match code {
            b'Z' => Value::Bool(self.read_u8()? != 0),
            b'B' => Value::Int(self.read_u8()? as i64),
            b'S' => Value::Int(self.read_u16()? as i64),
            b'I' => Value::Int(self.read_i32()? as i64),
            b'J' => Value::Int(self.read_i64()?),
            b'F' => Value::Int(self.read_u32()? as i64),
            b'D' => Value::Int(self.read_i64()?),
            b'C' => Value::Int(self.read_u16()? as i64),
            _ => return Err(RmclError::other("未知基础字段类型")),
        })
    }

    /// 从解析出的顶层 HashMap 中提取服务器列表
    fn collect_servers(&self, value: &Value) -> Result<Vec<ImportedServer>, RmclError> {
        let mut out = Vec::new();
        self.collect_pairs(value, &[], &mut out);
        Ok(out)
    }

    // 递归遍历对象/数组,把 (对象, 含 name/ip/port 的子对象) 当作一条服务器记录
    fn collect_pairs(&self, value: &Value, _path: &[String], out: &mut Vec<ImportedServer>) {
        match value {
            Value::Obj(fields) => {
                // 服务器条目:对象数组元素或 map 值。键(name)已在上一级丢失,但 ServerData
                // 自身带 name 字段,可直接取。
                if has(fields, "ip") && has(fields, "port") {
                    let name = str_field(fields, "name").unwrap_or_default();
                    let address = str_field(fields, "ip").unwrap_or_default();
                    let port = match int_field(fields, "port") {
                        Ok(p) => p,
                        Err(_) => 25565,
                    };
                    if !address.is_empty() {
                        out.push(ImportedServer {
                            name: if name.is_empty() {
                                address.clone()
                            } else {
                                name
                            },
                            address,
                            port: port_max(port),
                        });
                    }
                } else {
                    for (_, v) in fields {
                        self.collect_pairs(v, _path, out);
                    }
                }
            }
            Value::Arr(items) => {
                for it in items {
                    self.collect_pairs(it, _path, out);
                }
            }
            _ => {}
        }
    }
}

fn has(fields: &[(String, Value)], name: &str) -> bool {
    fields.iter().any(|(k, _)| k == name)
}

fn str_field(fields: &[(String, Value)], name: &str) -> Option<String> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::Str(s) => Some(s.clone()),
            _ => None,
        })
}

fn int_field(fields: &[(String, Value)], name: &str) -> Result<i64, RmclError> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::Int(n) => Some(*n),
            _ => None,
        })
        .ok_or_else(|| RmclError::other("服务器条目缺少端口字段"))
}

fn port_max(p: i64) -> u16 {
    p.clamp(1, 65535) as u16
}

fn push_char(utf8: &mut Vec<u8>, code: u32) {
    if code == 0 {
        utf8.push(b'?');
    } else if code <= 0x7F {
        utf8.push(code as u8);
    } else if code <= 0x7FF {
        utf8.push(0xC0 | (code >> 6) as u8);
        utf8.push(0x80 | (code & 0x3F) as u8);
    } else {
        utf8.push(0xE0 | (code >> 12) as u8);
        utf8.push(0x80 | ((code >> 6) & 0x3F) as u8);
        utf8.push(0x80 | (code & 0x3F) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_serialization_header() {
        assert!(parse_servers(b"not a servers.dat").is_err());
    }

    #[test]
    fn rejects_truncated() {
        assert!(parse_servers(&[0xAC, 0xED]).is_err());
    }

    // 构造一个最简的 HashMap(classDesc 无字段) + 一条 ServerData 对象的字节流。
    // 该流匹配本读取器对"对象字段按 classDesc 声明读取"的约定,用于验证字段提取。
    #[test]
    fn extracts_single_server() {
        let raw = build_demo_stream();
        let servers = parse_servers(&raw).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].address, "play.example.com");
        assert_eq!(servers[0].port, 25565);
    }

    fn build_demo_stream() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&STREAM_MAGIC);
        // HashMap object
        v.push(TC_OBJECT);
        push_classdesc(&mut v, "java.util.HashMap", &[]);
        // HashMap 内容:size + (key,value)
        v.extend_from_slice(&1i32.to_be_bytes());
        push_tc_string(&mut v, "my_server");
        // value = ServerData object
        v.push(TC_OBJECT);
        push_classdesc(
            &mut v,
            "net.minecraft.client.multiplayer.ServerData",
            &[
                ("acceptTextures", 'Z'),
                ("hidden", 'Z'),
                ("ping", 'I'),
                ("port", 'I'),
                ("icon", 'L'),
                ("ip", 'L'),
                ("name", 'L'),
            ],
        );
        // ServerData 字段值(与字段声明同序)
        v.push(0); // acceptTextures=false
        v.push(0); // hidden=false
        v.extend_from_slice(&0i32.to_be_bytes()); // ping
        v.extend_from_slice(&25565i32.to_be_bytes()); // port
        v.push(TC_NULL); // icon=null
        push_tc_string(&mut v, "play.example.com"); // ip
        push_tc_string(&mut v, "My Server"); // name
        v
    }

    // 写一个无字段/带字段的 classDesc
    fn push_classdesc(v: &mut Vec<u8>, name: &str, fields: &[(&str, char)]) {
        v.push(TC_CLASSDESC);
        push_str(v, name);
        v.extend_from_slice(&0i64.to_be_bytes()); // serialVersionUID
        v.push(0x02); // SC_SERIALIZABLE
        v.extend_from_slice(&(fields.len() as u16).to_be_bytes());
        for (fname, typecode) in fields {
            push_str(v, fname);
            v.push(*typecode as u8);
            if *typecode == 'L' {
                push_str(v, "java/lang/String;");
            }
        }
        v.push(TC_ENDBLOCKDATA);
        v.push(TC_NULL); // superclass
    }

    fn push_tc_string(v: &mut Vec<u8>, s: &str) {
        v.push(TC_STRING);
        push_str(v, s);
    }

    fn push_str(v: &mut Vec<u8>, s: &str) {
        v.extend_from_slice(&(s.len() as u16).to_be_bytes());
        v.extend_from_slice(s.as_bytes());
    }
}
