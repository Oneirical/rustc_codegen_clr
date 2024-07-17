use crate::asm::AssemblyExternRef;
use crate::asm_exporter::{AssemblyExportError, AssemblyExporter, AssemblyInfo};
use crate::method::Method;
use crate::type_def::TypeDef;

mod method;
use crate::{escape_type_name, DepthSetting};
use crate::{r#type::Type, IString};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use std::process::Command;
use std::{borrow::Cow, io::Write};
mod varaible;
pub struct CExporter {
    types: Vec<u8>,
    type_defs: Vec<u8>,
    method_defs: String,
    static_defs: Vec<u8>,
    encoded_asm: String,
    headers: Vec<u8>,
    defined: FxHashSet<IString>,
    delayed_typedefs: FxHashMap<IString, TypeDef>,
}
impl CExporter {
    pub fn init(_asm_info: &AssemblyInfo) -> Self {
        use std::fmt::Write;
        let mut encoded_asm = String::with_capacity(0x1_00);
        let types = Vec::with_capacity(0x1_00);
        let type_defs = Vec::with_capacity(0x1_00);
        let method_defs = String::with_capacity(0x1_00);
        let static_defs = Vec::with_capacity(0x1_00);
        let mut headers = Vec::with_capacity(0x1_00);
        write!(headers, "/*  This file was autogenerated by `rustc_codegen_clr` by FractalFir\n It contains C code made from Rust.*/\n").expect("Write error!");

        write!(
            headers,
            "#include  <stdint.h>\n#include <stdbool.h>\n#include <stddef.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <mm_malloc.h>\n#include <sys/syscall.h>\n #include<math.h>"
        )
        .expect("Write error!");
        headers.write_all(include_bytes!("c_header.h")).unwrap();
        writeln!(headers).expect("Write error!");
        writeln!(
            encoded_asm,
            "#pragma GCC diagnostic ignored \"-Wmaybe-uninitialized\""
        )
        .unwrap();
        writeln!(
            encoded_asm,
            "#pragma GCC diagnostic ignored \"-Wunused-label\""
        )
        .unwrap();
        writeln!(
            encoded_asm,
            "#pragma GCC diagnostic ignored \"-Wunused-but-set-variable\""
        )
        .unwrap();
        writeln!(
            encoded_asm,
            "#pragma GCC diagnostic ignored \"-Wunused-variable\""
        )
        .unwrap();
        writeln!(
            encoded_asm,
            "#pragma GCC diagnostic ignored \"-Wpointer-sign\""
        )
        .unwrap();
        Self {
            types,
            type_defs,
            encoded_asm,
            method_defs,
            static_defs,
            headers,
            defined: FxHashSet::with_hasher(FxBuildHasher::default()),
            delayed_typedefs: FxHashMap::with_hasher(FxBuildHasher::default()),
        }
    }
}

impl CExporter {
    fn as_source(&self, is_dll: bool) -> Vec<u8> {
        let mut res = self.headers.clone();
        res.extend(&self.types);
        res.extend(&self.type_defs);
        res.extend(self.method_defs.as_bytes());
        res.extend(&self.static_defs);
        res.extend(self.encoded_asm.as_bytes());
        if !is_dll {
            writeln!(res, "int main(int argc,char** argv){{_cctor();exec_fname = argv[0];entrypoint(argv + 1);}}").unwrap();
        }
        res
    }
    fn add_method_inner(&mut self, method: &Method, class: Option<&str>) {
        /*//eprintln!("C source:\n{}",String::from_utf8_lossy(&self.as_source()));
        let sig = method.sig();

        let name = method.name().replace('.', "_");
        // Puts is already defined in C.
        if name == "puts"
            || name == "malloc"
            || name == "printf"
            || name == "free"
            || name == "realloc"
            || name == "syscall"
        {
            return;
        }
        let output = c_tpe(sig.output());
        let mut inputs: String = "(".into();
        let mut input_iter = sig
            .inputs()
            .iter()
            .enumerate()
            .filter(|(_, tpe)| **tpe != Type::Void);
        if let Some((idx, input)) = input_iter.next() {
            inputs.push_str(&format!("{input} A{idx}", input = c_tpe(input)));
        }
        for (idx, input) in input_iter {
            inputs.push_str(&format!(",{input} A{idx} ", input = c_tpe(input)));
        }
        inputs.push(')');
        let mut code = String::new();
        for (id, (_, local)) in method.locals().iter().enumerate() {
            if *local == Type::Void {
                continue;
            }
            code.push_str(&format!("\t{local} L{id};\n", local = c_tpe(local)));
        }
        for bb in method.blocks() {
            code.push_str(&format!("\tBB_{}:\n", bb.id()));
            for tree in bb.trees() {
                code.push_str(&format!("{}\n", tree_string(tree, method)));
                //code.push_str(&format!("/*{tree:?}*/
        \n"));
            }
        }
        if let Some(class) = class {
            let class = escape_type_name(class);
            writeln!(self.method_defs, "{output} {class}{name} {inputs};").unwrap();
            write!(
                self.encoded_asm,
                "{output} {class}{name} {inputs}{{\n{code}}}\n"
            )
            .unwrap();
        } else {
            writeln!(self.method_defs, "{output} {name} {inputs};").unwrap();
            write!(self.encoded_asm, "{output} {name} {inputs}{{\n{code}}}\n").unwrap();
        }*/
        method::export_method(
            &mut self.method_defs,
            &mut self.encoded_asm,
            method,
            class,
            DepthSetting::with_pading(),
        )
        .unwrap();
    }
}
impl AssemblyExporter for CExporter {
    fn add_type(&mut self, tpe: &TypeDef) {
        let name: IString = escape_type_name(tpe.name()).into();
        if self.defined.contains(&name) {
            return;
        }
        for tpe_name in tpe
            .fields()
            .iter()
            .filter_map(|field| field.1.as_dotnet())
            .filter_map(|tpe| {
                if tpe.asm().is_none() {
                    Some(escape_type_name(tpe.name_path()))
                } else {
                    None
                }
            })
        {
            if !self.defined.contains::<IString>(&tpe_name.clone().into()) {
                //eprintln!("type {tpe_name:?} has unresolved dependencies");
                self.delayed_typedefs.insert(name, tpe.clone());
                return;
            }
        }
        let mut fields = String::new();
        if let Some(offsets) = tpe.explicit_offsets() {
            for ((field_name, field_type), offset) in tpe.fields().iter().zip(offsets) {
                if *field_type == Type::Void {
                    continue;
                }

                fields.push_str(&format!(
                    "\tstruct {{char pad[{offset}];{field_type} f;}} {field_name};\n\n",
                    field_type = c_tpe(field_type)
                ));
            }
        } else {
            for (field_name, field_type) in tpe.fields() {
                if *field_type == Type::Void {
                    continue;
                }
                fields.push_str(&format!(
                    "\tstruct {{{field_type} f;}} {field_name};\n",
                    field_type = c_tpe(field_type)
                ));
            }
        }
        for method in tpe.methods() {
            self.add_method_inner(method, Some(&name));
        }
        if tpe.explicit_offsets().is_some() {
            writeln!(self.types, "typedef union {name} {name};").unwrap();
            write!(self.type_defs, "union {name}{{\n{fields}}};\n").unwrap();
        } else {
            writeln!(self.types, "typedef struct {name} {name};").unwrap();
            write!(self.type_defs, "struct {name}{{\n{fields}}};\n").unwrap();
        }
        self.defined.insert(name);
        let delayed_typedefs = self.delayed_typedefs.clone();
        self.delayed_typedefs = FxHashMap::with_hasher(FxBuildHasher::default());
        for (_, tpe) in delayed_typedefs {
            self.add_type(&tpe);
        }
    }

    fn add_method(&mut self, method: &Method) {
        self.add_method_inner(method, None);
    }

    fn add_extern_method(
        &mut self,
        _lib_path: &str,
        name: &str,
        sig: &crate::FnSig,
        _preserve_errno: bool,
    ) {
        use std::fmt::Write;
        if name == "puts"
            || name == "malloc"
            || name == "printf"
            || name == "free"
            || name == "syscall"
            || name == "getenv"
            || name == "rename"
        {
            return;
        }
        let output = c_tpe(sig.output());
        let mut inputs: String = "(".into();
        let mut input_iter = sig
            .inputs()
            .iter()
            .enumerate()
            .filter(|(_, tpe)| **tpe != Type::Void);
        if let Some((idx, input)) = input_iter.next() {
            inputs.push_str(&format!("{input} A{idx}", input = c_tpe(input)));
        }
        for (idx, input) in input_iter {
            inputs.push_str(&format!(",{input} A{idx} ", input = c_tpe(input)));
        }
        inputs.push(')');
        writeln!(self.method_defs, "extern {output} {name} {inputs};").unwrap();
    }

    fn finalize(
        self,
        final_path: &std::path::Path,
        is_dll: bool,
    ) -> Result<(), AssemblyExportError> {
        let cc = "gcc";
        let src_path = final_path.with_extension("c");
        std::fs::File::create(&src_path)
            .unwrap()
            .write_all(&self.as_source(is_dll))
            .unwrap();
        let san_undef = false; //*crate::config::C_SANITIZE
        let sanitize = if san_undef {
            "-fsanitize=undefined"
        } else {
            "-O"
        };
        let out = Command::new(cc)
            .args([
                "-g",
                sanitize,
                "-o",
                final_path.to_string_lossy().as_ref(),
                src_path.to_string_lossy().as_ref(),
                "-lm",
                "-fno-strict-aliasing",
            ])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("error"), "C compiler error:{stderr:?}!");
        Ok(())
    }

    fn add_extern_ref(&mut self, _asm_name: &str, _info: &AssemblyExternRef) {
        // Not needed in C
    }

    fn add_global(&mut self, tpe: &crate::r#type::Type, name: &str, thread_local: bool) {
        writeln!(self.static_defs, "static {tpe} {name};", tpe = c_tpe(tpe)).unwrap();
    }
}
fn c_tpe(tpe: &Type) -> Cow<'static, str> {
    match tpe {
        Type::Bool => "bool".into(),
        Type::USize => "uintptr_t".into(),
        Type::ISize => "intptr_t".into(),
        Type::Void => "void".into(),
        Type::DotnetChar => "char".into(),
        Type::I128 => "__int128".into(),
        Type::U128 => "unsigned __int128".into(),
        Type::I64 => "int64_t".into(),
        Type::U64 => "uint64_t".into(),
        Type::I32 => "int32_t".into(),
        Type::U32 => "uint32_t".into(),
        Type::F64 => "float".into(),
        Type::F32 => "double".into(),
        Type::I16 => "int16_t".into(),
        Type::U16 => "uint16_t".into(),
        Type::I8 => "int8_t".into(),
        Type::U8 => "uint8_t".into(),
        Type::Ptr(inner) | Type::ManagedReference(inner) => {
            format!("{inner}*", inner = c_tpe(inner)).into()
        }
        Type::DotnetType(tref) => {
            if let Some(asm) = tref.asm() {
                match (asm, tref.name_path()) {
                    ("System.Runtime", "System.UInt128") => return c_tpe(&Type::U128),
                    ("System.Runtime", "System.Int128") => return c_tpe(&Type::I128),
                    _ => println!("Type {tref:?} is not supported in C"),
                }
            }
            if tref.is_valuetype() {
                escape_type_name(tref.name_path()).into()
            } else {
                format!("{name}*", name = escape_type_name(tref.name_path())).into()
            }
        }
        Type::DelegatePtr(_sig) => "void*".into(),
        Type::ManagedArray { element, dims } => {
            let ptrs: String = (0..(dims.get())).map(|_| '*').collect();
            format!("{element}{ptrs}", element = c_tpe(element)).into()
        }
        Type::Foreign => "Foregin".into(),
        _ => todo!("Unsuported type {tpe:?}"),
    }
}
